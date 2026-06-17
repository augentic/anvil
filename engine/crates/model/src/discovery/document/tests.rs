use super::*;

const SAMPLE: &str = "\
# Discovery

Some prose before the inventory.

## Lead inventory

### legacy:user-registration

- lead: user-registration
- source: legacy
- synopsis: Registration endpoint accepting email + password.

### legacy:password-reset-request

- lead: password-reset-request
- source: legacy
- synopsis: Reset endpoint.

## Notes

Some trailing prose.
";

#[test]
fn parses_canonical_layout() {
    let doc = Discovery::parse(SAMPLE).expect("parse ok");
    assert_eq!(doc.leads.len(), 2);
    assert_eq!(doc.leads[0].lead, "user-registration");
    assert_eq!(doc.leads[0].source, "legacy");
    assert_eq!(doc.leads[1].lead, "password-reset-request");
}

#[test]
fn parse_rejects_retired_aliases_bullet() {
    let err = Discovery::parse(
        "\
## Lead inventory

### legacy:user-registration

- lead: user-registration
- source: legacy
- aliases: [account-registration]
- synopsis: Registration endpoint.
",
    )
    .expect_err("aliases bullet must fail");
    match err {
        Error::Diag { code, detail } => {
            assert_eq!(code, "discovery-parse-failed");
            assert!(detail.contains("aliases:"), "{detail}");
        }
        other => panic!("expected Diag, got: {other:?}"),
    }
}

#[test]
fn accepts_headingless_blocks() {
    let doc = Discovery::parse_lead_set(
        "\
### user-registration

- lead: user-registration
- synopsis: Registration endpoint.
",
    )
    .expect("parse ok");

    assert_eq!(doc.leads.len(), 1);
    assert_eq!(doc.leads[0].lead, "user-registration");
    assert_eq!(doc.leads[0].source, "");
}

#[test]
fn accepts_existing_inventory_heading() {
    let lead_set = "\
## Lead inventory

### user-registration

- lead: user-registration
- synopsis: Registration endpoint.
";
    let framed = Discovery::parse(lead_set).expect("parse ok");
    let lead_set = Discovery::parse_lead_set(lead_set).expect("parse lead set ok");

    assert_eq!(lead_set, framed);
}

#[test]
fn accepts_whitespace_only_content() {
    let doc = Discovery::parse_lead_set("\n  \n").expect("parse ok");

    assert!(doc.leads.is_empty());
}

#[test]
fn round_trips_byte_stable_when_unchanged() {
    let doc = Discovery::parse(SAMPLE).expect("parse ok");
    let rendered = doc.render();
    let reparsed = Discovery::parse(&rendered).expect("reparse ok");
    assert_eq!(doc.leads, reparsed.leads);
}

#[test]
fn parses_optional_topics_bullet() {
    let doc = Discovery::parse(
        "\
## Lead inventory

### legacy:user-registration

- lead: user-registration
- source: legacy
- synopsis: Registration endpoint.
- topics: [identity, account-creation, validation]
",
    )
    .expect("parse ok");
    assert_eq!(doc.leads[0].topics, ["identity", "account-creation", "validation"]);
}

#[test]
fn topics_round_trip_byte_stable() {
    let doc = Discovery::parse(
        "\
## Lead inventory

### legacy:user-registration

- lead: user-registration
- source: legacy
- synopsis: Registration endpoint.
- topics: [identity, validation]
",
    )
    .expect("parse ok");
    let reparsed = Discovery::parse(&doc.render()).expect("reparse ok");
    assert_eq!(doc.leads, reparsed.leads);
    assert_eq!(reparsed.leads[0].topics, ["identity", "validation"]);
}

#[test]
fn resolve_lead_matches_id() {
    let doc = Discovery::parse(SAMPLE).expect("parse ok");
    let hit = doc.resolve_lead("user-registration").expect("resolves");
    assert_eq!(hit.lead, "user-registration");
}

#[test]
fn resolve_lead_unknown_errors() {
    let doc = Discovery::parse(SAMPLE).expect("parse ok");
    let err = doc.resolve_lead("never-heard-of-it").expect_err("unknown errs");
    match err {
        ResolveError::Unknown { token } => assert_eq!(token, "never-heard-of-it"),
    }
}

fn lead(lead: &str, source: &str, synopsis: &str) -> Lead {
    Lead {
        lead: lead.to_string(),
        source: source.to_string(),
        synopsis: synopsis.to_string(),
        topics: Vec::new(),
    }
}

#[test]
fn merge_survey_replaces_same_id_block() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("discovery.md");
    let mut doc = Discovery::parse(SAMPLE).expect("parse ok");

    let incoming =
        vec![lead("user-registration", "legacy", "Registration endpoint (re-surveyed).")];
    doc.merge_survey("legacy", incoming, &path).expect("merge ok");

    let reloaded = Discovery::load(&path).expect("reload ok");
    let hit = reloaded.leads.iter().find(|c| c.lead == "user-registration").expect("present");
    assert_eq!(hit.synopsis, "Registration endpoint (re-surveyed).");
    assert_eq!(
        reloaded.leads.iter().filter(|c| c.lead == "user-registration").count(),
        1,
        "replaced in place, not duplicated"
    );
    assert!(
        reloaded.leads.iter().any(|c| c.lead == "password-reset-request"),
        "leads absent from the incoming set survive untouched"
    );
}

#[test]
fn merge_preserves_ordering() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("discovery.md");
    let doc_md = "\
## Lead inventory

### legacy:x

- lead: x
- source: legacy
- synopsis: X.

### legacy:y

- lead: y
- source: legacy
- synopsis: Y.

### legacy:z

- lead: z
- source: legacy
- synopsis: Z.
";
    let mut doc = Discovery::parse(doc_md).expect("parse ok");

    let incoming = vec![lead("y", "legacy", "Y (re-surveyed)."), lead("w", "legacy", "W (new).")];
    doc.merge_survey("legacy", incoming, &path).expect("merge ok");

    let reloaded = Discovery::load(&path).expect("reload ok");
    let ids: Vec<&str> = reloaded.leads.iter().map(|c| c.lead.as_str()).collect();
    assert_eq!(ids, vec!["x", "y", "z", "w"]);
}
