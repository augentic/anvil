//! RFC-96 D8 domain rounds: closed-DTO round-trip and rejection,
//! per-target persistence with identity reuse, the drain-gate
//! predicate, and the protected-input closure algebra.

use std::collections::{BTreeMap, BTreeSet};

use project::config::Layout;
use project::domain::{
    Closure, DomainRound, RoundKind, VERSION, Verdict, complete_passed, protected_closure,
};
use project::journal::FactEpochRef;
use project::plan::decomposition::{Covered, CoveredKind, Oracle};
use project::snapshot::SnapshotId;

const fn layout(root: &std::path::Path) -> Layout<'_> {
    Layout::new(root)
}

fn cid(fill: char) -> SnapshotId {
    SnapshotId::from_digest(&fill.to_string().repeat(64))
}

fn round(domain: &str, kind: RoundKind, verdict: Verdict, target: &str) -> DomainRound {
    DomainRound {
        version: VERSION,
        domain: domain.to_string(),
        kind,
        verdict,
        targets: vec![target.to_string()],
        revision: cid('1'),
        authorization: FactEpochRef {
            writer: "writer-a".into(),
            sequence: 3,
        },
        bases: BTreeMap::from([(target.to_string(), cid('2'))]),
        children: vec![cid('3')],
        waves: vec![cid('4')],
        results: BTreeMap::from([(target.to_string(), cid('5'))]),
        protected_inputs: Closure::default().digest().expect("closure digest"),
        verification_report: Some(cid('6')),
    }
}

mod dto {
    use super::*;

    // Canonical YAML round-trips byte-identically and the content
    // digest is stable across re-serialization.
    #[test]
    fn round_trip() {
        let original = round("root", RoundKind::Frontier, Verdict::Passed, "app");
        let yaml = original.canonical_yaml().expect("yaml");
        let parsed: DomainRound = serde_saphyr::from_str(&yaml).expect("parse");
        assert_eq!(parsed, original);
        assert_eq!(parsed.digest().expect("digest"), original.digest().expect("digest"));
    }

    // The DTO is closed: an unknown field rejects the document.
    #[test]
    fn unknown_field_rejected() {
        let mut yaml =
            round("root", RoundKind::Complete, Verdict::Passed, "app").canonical_yaml().expect("");
        yaml.push_str("surprise: true\n");
        serde_saphyr::from_str::<DomainRound>(&yaml).unwrap_err();
    }

    // `same_key` binds identity fields only: verdict and the
    // verification report may differ, everything else may not.
    #[test]
    fn same_key_ignores_verdict() {
        let passed = round("root", RoundKind::Complete, Verdict::Passed, "app");
        let mut failed = passed.clone();
        failed.verdict = Verdict::Failed;
        failed.verification_report = None;
        assert!(passed.same_key(&failed));
        let mut other = passed.clone();
        other.children = vec![cid('9')];
        assert!(!passed.same_key(&other));
    }
}

mod persistence {
    use super::*;

    // A round persists under every bound target; `find` reuses it by
    // identity and an identical re-write is idempotent.
    #[test]
    fn write_find_reuse() {
        let dir = tempfile::tempdir().expect("tempdir");
        let layout = layout(dir.path());
        let mut multi = round("root", RoundKind::Complete, Verdict::Passed, "app");
        multi.targets = vec!["app".into(), "other".into()];
        let digest = multi.write(layout).expect("write");
        assert_eq!(multi.write(layout).expect("rewrite"), digest, "write-once idempotent");
        for target in ["app", "other"] {
            let found = DomainRound::find(layout, target, &multi).expect("find");
            assert_eq!(found.as_ref(), Some(&multi), "recorded under `{target}`");
        }
        assert!(
            DomainRound::find(
                layout,
                "app",
                &round("root", RoundKind::Frontier, Verdict::Passed, "app")
            )
            .expect("find")
            .is_none(),
            "a different identity finds nothing"
        );
    }

    // The drain gate: only a *passed* complete round for the root at
    // exactly the current revision and accepted CID passes; a target
    // with no accepted CID gates nothing.
    #[test]
    fn drain_gate() {
        let dir = tempfile::tempdir().expect("tempdir");
        let layout = layout(dir.path());
        let accepted = cid('5');
        assert!(complete_passed(layout, "root", &cid('1'), "app", None).expect("no cid"));
        assert!(
            !complete_passed(layout, "root", &cid('1'), "app", Some(&accepted)).expect("empty"),
            "no round recorded"
        );
        round("root", RoundKind::Complete, Verdict::Failed, "app").write(layout).expect("failed");
        assert!(
            !complete_passed(layout, "root", &cid('1'), "app", Some(&accepted)).expect("failed"),
            "a failed round does not pass"
        );
        round("root", RoundKind::Complete, Verdict::Passed, "app").write(layout).expect("passed");
        assert!(complete_passed(layout, "root", &cid('1'), "app", Some(&accepted)).expect("pass"));
        assert!(
            !complete_passed(layout, "root", &cid('7'), "app", Some(&accepted)).expect("rev"),
            "a different revision does not pass"
        );
        assert!(
            !complete_passed(layout, "root", &cid('1'), "app", Some(&cid('8'))).expect("cid"),
            "a different accepted CID does not pass"
        );
    }
}

mod closure {
    use super::*;

    fn file(path: &str) -> Covered {
        Covered {
            kind: CoveredKind::File,
            path: path.to_string(),
        }
    }

    fn tree(path: &str) -> Covered {
        Covered {
            kind: CoveredKind::Tree,
            path: path.to_string(),
        }
    }

    fn oracle(id: &str, fill: char) -> Oracle {
        Oracle {
            id: id.to_string(),
            digest: cid(fill),
        }
    }

    fn touched(paths: &[&str]) -> BTreeSet<String> {
        paths.iter().map(ToString::to_string).collect()
    }

    /// One algebra case: descendants' declared sets, the touched
    /// paths, and the expected closure.
    type Case<'a> = (&'a str, Vec<(&'a [Covered], &'a [Oracle])>, BTreeSet<String>, Closure);

    // The dense algebra matrix: exact intersection over descendants,
    // file/tree touch removal, identical-(id, digest) oracle
    // intersection, and canonical empty encodings.
    #[test]
    fn algebra() {
        let a_cov = [file("docs/spec.md"), tree("contracts"), file("README.md")];
        let b_cov = [file("docs/spec.md"), tree("contracts")];
        let a_or = [oracle("ci", 'a'), oracle("license", 'b')];
        let b_or = [oracle("ci", 'a'), oracle("license", 'c')];

        let cases: Vec<Case<'_>> = vec![
            ("no descendants", vec![], touched(&[]), Closure::default()),
            (
                "single descendant is its own closure",
                vec![(&a_cov, &a_or)],
                touched(&[]),
                Closure {
                    covered: a_cov.to_vec(),
                    oracles: a_or.to_vec(),
                },
            ),
            (
                "intersection drops the unshared entry and the drifted oracle",
                vec![(&a_cov, &a_or), (&b_cov, &b_or)],
                touched(&[]),
                Closure {
                    covered: b_cov.to_vec(),
                    oracles: vec![oracle("ci", 'a')],
                },
            ),
            (
                "a touched file leaves the closure",
                vec![(&b_cov, &[])],
                touched(&["docs/spec.md"]),
                Closure {
                    covered: vec![tree("contracts")],
                    oracles: Vec::new(),
                },
            ),
            (
                "a path under a tree entry invalidates the tree",
                vec![(&b_cov, &[])],
                touched(&["contracts/api.yaml"]),
                Closure {
                    covered: vec![file("docs/spec.md")],
                    oracles: Vec::new(),
                },
            ),
            (
                "a sibling prefix does not invalidate the tree",
                vec![(&b_cov, &[])],
                touched(&["contracts-v2/api.yaml"]),
                Closure {
                    covered: b_cov.to_vec(),
                    oracles: Vec::new(),
                },
            ),
            (
                "an empty descendant empties the intersection",
                vec![(&a_cov, &a_or), (&[], &[])],
                touched(&[]),
                Closure::default(),
            ),
        ];
        for (name, descendants, touched, expected) in cases {
            let mut sorted = expected;
            sorted.covered.sort();
            sorted.oracles.sort();
            assert_eq!(protected_closure(&descendants, &touched), sorted, "{name}");
        }
    }

    // Empty and absent sets encode canonically: the default closure's
    // digest is total and stable.
    #[test]
    fn empty_digest_total() {
        assert_eq!(
            Closure::default().digest().expect("digest"),
            protected_closure(&[], &BTreeSet::new()).digest().expect("digest"),
        );
    }
}
