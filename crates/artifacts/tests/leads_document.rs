//! `leads.md` catalog parser, digest, and merge (`artifacts::leads`).

use artifacts::leads::document::{Leads, ResolveError};
use artifacts::leads::lead::Lead;
use error::Error;

const SAMPLE: &str = "\
## Lead inventory

### legacy:user-registration

- lead: user-registration
- source: legacy
- synopsis: Registration endpoint accepting email + password.

### legacy:password-reset-request

- lead: password-reset-request
- source: legacy
- synopsis: Reset endpoint.
";

fn lead(id: &str, source: &str, synopsis: &str) -> Lead {
    Lead::new(id, source, synopsis)
}

mod parse {
    use super::*;

    #[test]
    fn catalog_only() {
        let doc = Leads::parse(SAMPLE).expect("parse ok");
        assert_eq!(doc.leads().len(), 2);
        assert_eq!(doc.leads()[0].lead, "user-registration");
        assert_eq!(doc.leads()[0].source, "legacy");
        assert_eq!(doc.leads()[1].lead, "password-reset-request");
    }

    #[test]
    fn rejects_preamble() {
        let err = Leads::parse("# Discovery\n\n## Lead inventory\n").expect_err("preamble");
        match err {
            Error::Diag { code, detail } => {
                assert_eq!(code, "leads-parse-failed");
                assert!(detail.contains("catalog-only"), "{detail}");
            }
            other => panic!("expected Diag, got: {other:?}"),
        }
    }

    #[test]
    fn rejects_suffix() {
        let err = Leads::parse(
            "## Lead inventory\n\n### legacy:x\n\n- lead: x\n- source: legacy\n- synopsis: X.\n\n## Notes\n",
        )
        .expect_err("suffix");
        match err {
            Error::Diag { code, .. } => assert_eq!(code, "leads-parse-failed"),
            other => panic!("expected Diag, got: {other:?}"),
        }
    }

    #[test]
    fn rejects_retired_aliases() {
        let err = Leads::parse(
            "## Lead inventory\n\n### legacy:user-registration\n\n- lead: user-registration\n\
             - source: legacy\n- aliases: [account-registration]\n- synopsis: Registration.\n",
        )
        .expect_err("aliases");
        match err {
            Error::Diag { code, detail } => {
                assert_eq!(code, "leads-parse-failed");
                assert!(detail.contains("aliases:"), "{detail}");
            }
            other => panic!("expected Diag, got: {other:?}"),
        }
    }

    #[test]
    fn accepts_headingless() {
        let doc = Leads::parse_lead_set(
            "### user-registration\n\n- lead: user-registration\n- synopsis: Registration endpoint.\n",
        )
        .expect("parse ok");
        assert_eq!(doc.leads().len(), 1);
        assert_eq!(doc.leads()[0].lead, "user-registration");
        assert_eq!(doc.leads()[0].source, "");
    }

    #[test]
    fn accepts_inventory_heading() {
        let lead_set = "## Lead inventory\n\n### user-registration\n\n- lead: user-registration\n- synopsis: Registration.\n";
        let framed = Leads::parse(lead_set).expect("parse");
        let lead_set = Leads::parse_lead_set(lead_set).expect("parse lead set");
        assert_eq!(lead_set, framed);
    }

    #[test]
    fn accepts_whitespace_only() {
        let doc = Leads::parse_lead_set("\n  \n").expect("parse ok");
        assert!(doc.leads().is_empty());
    }

    #[test]
    fn parent_and_focus() {
        let doc = Leads::parse(
            "## Lead inventory\n\n### code:orders-create\n\n- lead: orders-create\n\
             - source: code\n- synopsis: Create order.\n- parent: orders-api\n- focus: POST /orders\n",
        )
        .expect("parse ok");
        assert_eq!(doc.leads()[0].parent.as_deref(), Some("orders-api"));
        assert_eq!(doc.leads()[0].focus.as_deref(), Some("POST /orders"));
    }
}

mod round_trip {
    use super::*;

    #[test]
    fn no_preamble() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("leads.md");
        let doc = Leads::parse(SAMPLE).expect("parse ok");
        doc.write_atomic(&path).expect("write ok");
        let reparsed = Leads::load(&path).expect("reload ok");
        assert_eq!(doc.leads(), reparsed.leads());
        let raw = std::fs::read_to_string(&path).expect("read");
        assert!(raw.starts_with("## Lead inventory\n"), "{raw}");
        assert!(!raw.contains("# Discovery"), "{raw}");
    }
}

mod digest {
    use super::*;

    #[test]
    fn formatting_stable() {
        let compact = Leads::parse(SAMPLE).expect("parse");
        let spaced = Leads::parse(
            "## Lead inventory\n\n\n### legacy:user-registration\n\n- lead: user-registration\n\
             - source: legacy\n- synopsis: Registration endpoint accepting email + password.\n\n\n\
             ### legacy:password-reset-request\n\n- lead: password-reset-request\n- source: legacy\n\
             - synopsis: Reset endpoint.\n",
        )
        .expect("parse spaced");
        assert_eq!(compact.digest_hex().expect("a"), spaced.digest_hex().expect("b"));
    }

    #[test]
    fn edit_invalidates() {
        let base = Leads::parse(SAMPLE).expect("parse");
        let digest = base.digest_hex().expect("digest");

        let mut source = base.clone();
        source.lead_mut("user-registration").expect("lead").source = "other".into();
        assert_ne!(digest, source.digest_hex().expect("source"));

        let mut id = base.clone();
        id.lead_mut("user-registration").expect("lead").lead = "account-registration".into();
        assert_ne!(digest, id.digest_hex().expect("id"));

        let mut synopsis = base.clone();
        synopsis.lead_mut("user-registration").expect("lead").synopsis = "Changed.".into();
        assert_ne!(digest, synopsis.digest_hex().expect("synopsis"));

        let mut topics = base.clone();
        topics.lead_mut("user-registration").expect("lead").topics = vec!["identity".into()];
        assert_ne!(digest, topics.digest_hex().expect("topics"));

        let mut parent = base.clone();
        parent.lead_mut("user-registration").expect("lead").parent = Some("accounts".into());
        assert_ne!(digest, parent.digest_hex().expect("parent"));

        let mut focus = base;
        focus.lead_mut("user-registration").expect("lead").focus = Some("POST /register".into());
        assert_ne!(digest, focus.digest_hex().expect("focus"));
    }
}

mod topics {
    use super::*;

    #[test]
    fn optional_bullet() {
        let doc = Leads::parse(
            "## Lead inventory\n\n### legacy:user-registration\n\n- lead: user-registration\n\
             - source: legacy\n- synopsis: Registration endpoint.\n- topics: [identity, account-creation]\n",
        )
        .expect("parse ok");
        assert_eq!(doc.leads()[0].topics, ["identity", "account-creation"]);
    }

    #[test]
    fn round_trip_byte_stable() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("leads.md");
        let doc = Leads::parse(
            "## Lead inventory\n\n### legacy:user-registration\n\n- lead: user-registration\n\
             - source: legacy\n- synopsis: Registration endpoint.\n- topics: [identity, validation]\n",
        )
        .expect("parse ok");
        doc.write_atomic(&path).expect("write ok");
        let reparsed = Leads::load(&path).expect("reload ok");
        assert_eq!(doc.leads(), reparsed.leads());
        assert_eq!(reparsed.leads()[0].topics, ["identity", "validation"]);
    }
}

mod resolve {
    use super::*;

    #[test]
    fn id_match() {
        let doc = Leads::parse(SAMPLE).expect("parse ok");
        let hit = doc.resolve_lead("user-registration").expect("resolves");
        assert_eq!(hit.lead, "user-registration");
    }

    #[test]
    fn unknown_errors() {
        let doc = Leads::parse(SAMPLE).expect("parse ok");
        let err = doc.resolve_lead("never-heard-of-it").expect_err("unknown errs");
        match err {
            ResolveError::Unknown { token } => assert_eq!(token, "never-heard-of-it"),
        }
    }
}

mod merge {
    use super::*;

    #[test]
    fn survey_replaces_id_block() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("leads.md");
        let mut doc = Leads::parse(SAMPLE).expect("parse ok");

        let incoming =
            vec![lead("user-registration", "legacy", "Registration endpoint (re-surveyed).")];
        doc.merge_survey("legacy", incoming, &path).expect("merge ok");

        let reloaded = Leads::load(&path).expect("reload ok");
        let hit = reloaded.leads().iter().find(|c| c.lead == "user-registration").expect("present");
        assert_eq!(hit.synopsis, "Registration endpoint (re-surveyed).");
        assert_eq!(reloaded.leads().iter().filter(|c| c.lead == "user-registration").count(), 1);
        assert!(reloaded.leads().iter().any(|c| c.lead == "password-reset-request"));
    }

    #[test]
    fn preserves_ordering() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("leads.md");
        let mut doc = Leads::parse(
            "## Lead inventory\n\n### legacy:x\n\n- lead: x\n- source: legacy\n- synopsis: X.\n\n\
             ### legacy:y\n\n- lead: y\n- source: legacy\n- synopsis: Y.\n\n\
             ### legacy:z\n\n- lead: z\n- source: legacy\n- synopsis: Z.\n",
        )
        .expect("parse ok");

        let incoming =
            vec![lead("y", "legacy", "Y (re-surveyed)."), lead("w", "legacy", "W (new).")];
        doc.merge_survey("legacy", incoming, &path).expect("merge ok");

        let reloaded = Leads::load(&path).expect("reload ok");
        let ids: Vec<&str> = reloaded.leads().iter().map(|c| c.lead.as_str()).collect();
        assert_eq!(ids, vec!["x", "y", "z", "w"]);
    }
}
