//! Definition-home declared inputs (RFC-104 D1/D2): fail-closed
//! loads, typed validation, canonical digests, and the surgical
//! coverage persist contract.

use std::collections::BTreeMap;
use std::path::Path;

use project::snapshot::SnapshotId;
use system::{
    Coverage, Disposition, Layout, RowPatch, Scope, SurveyError, SurveyErrorKind, coverage,
};

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    std::fs::write(path, text).expect("write");
}

fn cid(seed: u8) -> SnapshotId {
    SnapshotId::from_digest(&format!("{:064x}", u128::from(seed)))
}

const SCOPE: &str = "\
version: 1
id: orders
decision: Recover order-taking architecture for a first migration wave
products: [orders]
journeys: [place-order]
environments: [prod]
organizations: [commerce]
";

const COVERAGE: &str = "\
version: 1
candidates:
  - key: orders-code
    location: https://github.com/acme/orders
    adapter: typescript
    disposition: included
    reason: Primary order service repository
  - key: ops-runbook
    location: https://wiki.acme.example/runbook
    disposition: inaccessible
    reason: Wiki export refused by the client
";

mod scope {
    use super::*;

    #[test]
    fn loads() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = Layout::new(dir.path()).scope_path();
        write(&path, SCOPE);
        let scope = Scope::load(&path).expect("load");
        assert_eq!(scope.id, "orders");
        assert_eq!(scope.products, ["orders"]);
    }

    #[test]
    fn missing_fails_closed() {
        let dir = tempfile::tempdir().expect("tempdir");
        let err = Scope::load(&Layout::new(dir.path()).scope_path()).expect_err("missing");
        assert_eq!(err.variant_str(), "system-scope-missing");
        assert!(err.hint().expect("hint").contains("coverage.yaml"), "hint prints the template");
    }

    #[test]
    fn unknown_field_rejected() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = Layout::new(dir.path()).scope_path();
        write(&path, &format!("{SCOPE}locators: [nope]\n"));
        let err = Scope::load(&path).expect_err("unknown field");
        assert_eq!(err.variant_str(), "yaml");
    }

    #[test]
    fn version_gate() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = Layout::new(dir.path()).scope_path();
        write(&path, &SCOPE.replace("version: 1", "version: 2"));
        let err = Scope::load(&path).expect_err("bad version");
        assert_eq!(err.variant_str(), "system-scope-invalid");
    }

    #[test]
    fn digest_ignores_formatting() {
        let dir = tempfile::tempdir().expect("tempdir");
        let layout = Layout::new(dir.path());
        write(&layout.scope_path(), SCOPE);
        let a = Scope::load(&layout.scope_path()).expect("load a").digest().expect("digest a");

        // Same declared content, different flow style and key order.
        let reordered = "\
id: orders
version: 1
products:
  - orders
journeys: [place-order]
environments: [prod]
organizations: [commerce]
decision: Recover order-taking architecture for a first migration wave
";
        write(&layout.scope_path(), reordered);
        let b = Scope::load(&layout.scope_path()).expect("load b").digest().expect("digest b");
        assert_eq!(a, b, "digest is canonical, not formatting-bound");
    }
}

mod coverage_rows {
    use super::*;

    #[test]
    fn loads() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = Layout::new(dir.path()).coverage_path();
        write(&path, COVERAGE);
        let coverage = Coverage::load(&path).expect("load");
        assert_eq!(coverage.candidates.len(), 2);
        assert_eq!(coverage.included().count(), 1);
        let row = coverage.row("ops-runbook").expect("row");
        assert_eq!(row.disposition, Disposition::Inaccessible);
        assert!(row.adapter.is_none());
    }

    #[test]
    fn missing_fails_closed() {
        let dir = tempfile::tempdir().expect("tempdir");
        let err = Coverage::load(&Layout::new(dir.path()).coverage_path()).expect_err("missing");
        assert_eq!(err.variant_str(), "system-coverage-missing");
        assert!(err.hint().is_some(), "hint prints the template");
    }

    #[test]
    fn adapter_iff_included() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = Layout::new(dir.path()).coverage_path();

        // Included without adapter.
        write(&path, &COVERAGE.replace("    adapter: typescript\n", ""));
        let err = Coverage::load(&path).expect_err("included needs adapter");
        assert_eq!(err.variant_str(), "system-coverage-invalid");

        // Non-included with adapter.
        write(
            &path,
            &COVERAGE.replace(
                "    disposition: inaccessible\n",
                "    adapter: documentation\n    disposition: inaccessible\n",
            ),
        );
        let err = Coverage::load(&path).expect_err("non-included forbids adapter");
        assert_eq!(err.variant_str(), "system-coverage-invalid");
    }

    #[test]
    fn duplicate_key_rejected() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = Layout::new(dir.path()).coverage_path();
        let duplicated = format!(
            "{COVERAGE}  - key: orders-code\n    location: /elsewhere\n    disposition: excluded\n    reason: duplicate\n"
        );
        write(&path, &duplicated);
        let err = Coverage::load(&path).expect_err("duplicate key");
        assert_eq!(err.variant_str(), "system-coverage-invalid");
    }

    #[test]
    fn unknown_field_rejected() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = Layout::new(dir.path()).coverage_path();
        write(
            &path,
            &COVERAGE.replace("    reason: Primary", "    cid: nope\n    reason: Primary"),
        );
        let err = Coverage::load(&path).expect_err("unknown field");
        assert_eq!(err.variant_str(), "yaml");
    }

    #[test]
    fn survey_error_round_trips() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = Layout::new(dir.path()).coverage_path();
        let failed = format!(
            "{COVERAGE}  - key: billing-code\n    location: https://github.com/acme/billing\n    adapter: typescript\n    disposition: included\n    reason: Billing service repository\n    survey-error:\n      kind: access\n      detail: GitHub returned 404\n"
        );
        write(&path, &failed);
        let coverage = Coverage::load(&path).expect("load");
        let row = coverage.row("billing-code").expect("row");
        let error = row.survey_error.as_ref().expect("survey-error");
        assert_eq!(error.kind, SurveyErrorKind::Access);
    }
}

mod persist {
    use super::*;

    fn seeded(dir: &Path) -> std::path::PathBuf {
        let path = Layout::new(dir).coverage_path();
        write(&path, COVERAGE);
        path
    }

    #[test]
    fn success_clears_error() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = seeded(dir.path());

        // First run fails, second succeeds: the failure record must
        // not survive the success.
        let failed = BTreeMap::from([(
            "orders-code".to_string(),
            RowPatch::Failed(SurveyError {
                kind: SurveyErrorKind::Access,
                detail: "GitHub returned 404".to_string(),
            }),
        )]);
        coverage::persist(&path, &failed).expect("persist failure");

        let observed = BTreeMap::from([(
            "orders-code".to_string(),
            RowPatch::Observed {
                cid: cid(1),
                revision: Some("7f3a9c1d2e4b".to_string()),
            },
        )]);
        let after = coverage::persist(&path, &observed).expect("persist success");
        let row = after.row("orders-code").expect("row");
        assert_eq!(row.observed_cid, Some(cid(1)));
        assert_eq!(row.observed_revision.as_deref(), Some("7f3a9c1d2e4b"));
        assert!(row.survey_error.is_none(), "next success clears survey-error");
    }

    #[test]
    fn failure_keeps_observed() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = seeded(dir.path());

        let observed = BTreeMap::from([(
            "orders-code".to_string(),
            RowPatch::Observed {
                cid: cid(2),
                revision: None,
            },
        )]);
        coverage::persist(&path, &observed).expect("persist success");

        let failed = BTreeMap::from([(
            "orders-code".to_string(),
            RowPatch::Failed(SurveyError {
                kind: SurveyErrorKind::Adapter,
                detail: "survey answer failed the schema gate".to_string(),
            }),
        )]);
        let after = coverage::persist(&path, &failed).expect("persist failure");
        let row = after.row("orders-code").expect("row");
        assert_eq!(row.observed_cid, Some(cid(2)), "failure never clears a prior observed tree");
        assert_eq!(row.survey_error.as_ref().expect("error").kind, SurveyErrorKind::Adapter);
    }

    #[test]
    fn declared_fields_survive() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = seeded(dir.path());
        let before = Coverage::load(&path).expect("load before");

        let patches = BTreeMap::from([(
            "orders-code".to_string(),
            RowPatch::Observed {
                cid: cid(3),
                revision: None,
            },
        )]);
        let after = coverage::persist(&path, &patches).expect("persist");

        for (b, a) in before.candidates.iter().zip(&after.candidates) {
            assert_eq!(b.key, a.key);
            assert_eq!(b.location, a.location);
            assert_eq!(b.adapter, a.adapter);
            assert_eq!(b.disposition, a.disposition);
            assert_eq!(b.reason, a.reason);
        }
    }

    #[test]
    fn vanished_row_skipped() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = seeded(dir.path());
        let patches = BTreeMap::from([(
            "deleted-by-operator".to_string(),
            RowPatch::Observed {
                cid: cid(4),
                revision: None,
            },
        )]);
        let after = coverage::persist(&path, &patches).expect("persist");
        assert!(after.row("deleted-by-operator").is_none(), "persist invents no rows");
        assert_eq!(after.candidates.len(), 2);
    }

    #[test]
    fn untouched_rows_unchanged() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = seeded(dir.path());
        let patches = BTreeMap::from([(
            "orders-code".to_string(),
            RowPatch::Observed {
                cid: cid(5),
                revision: None,
            },
        )]);
        let after = coverage::persist(&path, &patches).expect("persist");
        let untouched = after.row("ops-runbook").expect("row");
        assert!(untouched.observed_cid.is_none());
        assert!(untouched.survey_error.is_none());

        // The rewrite is canonical and re-loadable.
        let reloaded = Coverage::load(&path).expect("reload");
        assert_eq!(reloaded, after);
    }
}
