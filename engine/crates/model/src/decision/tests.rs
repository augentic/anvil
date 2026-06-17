use super::*;

const VALID: &str = "---\n\
slug: identity-store-postgres\n\
status: accepted\n\
supersedes: [DEC-0003]\n\
related: [REQ-001, REQ-014]\n\
---\n\
# Use PostgreSQL for the identity store\n\
\n\
## Context\n\
Why the decision is needed.\n\
\n\
## Decision\n\
What was chosen.\n\
\n\
## Consequences\n\
Trade-offs.\n";

#[test]
fn parses_well_formed_record() {
    let parsed = parse_decision(VALID);
    assert!(parsed.findings.is_empty(), "no findings expected; got {:?}", parsed.findings);
    let record = parsed.record.expect("record parsed");
    assert_eq!(record.slug, "identity-store-postgres");
    assert_eq!(record.status, DecisionStatus::Accepted);
    assert_eq!(record.supersedes, vec!["DEC-0003"]);
    assert_eq!(record.related, vec!["REQ-001", "REQ-014"]);
    assert_eq!(parsed.title.as_deref(), Some("Use PostgreSQL for the identity store"));
}

#[test]
fn missing_frontmatter_is_schema_finding() {
    let parsed = parse_decision("# Title\n\n## Context\n## Decision\n## Consequences\n");
    assert!(parsed.record.is_none());
    assert!(parsed.findings.iter().any(|f| f.rule_id == "decision-record-schema"));
}

#[test]
fn unknown_field_is_schema_finding() {
    let text = "---\nslug: ok\nstatus: accepted\nbogus: nope\n---\n# T\n## Context\n## Decision\n## Consequences\n";
    let parsed = parse_decision(text);
    assert!(parsed.record.is_none());
    assert!(parsed.findings.iter().any(|f| f.rule_id == "decision-record-schema"));
}

#[test]
fn bad_status_enum_is_schema_finding() {
    let text = "---\nslug: ok\nstatus: maybe\n---\n# T\n## Context\n## Decision\n## Consequences\n";
    let parsed = parse_decision(text);
    assert!(parsed.record.is_none());
    assert!(parsed.findings.iter().any(|f| f.rule_id == "decision-record-schema"));
}

#[test]
fn missing_section_is_section_finding() {
    let text = "---\nslug: ok\nstatus: accepted\n---\n# T\n\n## Context\nx\n\n## Decision\ny\n";
    let parsed = parse_decision(text);
    let missing: Vec<_> =
        parsed.findings.iter().filter(|f| f.rule_id == "decision-record-section-missing").collect();
    assert_eq!(missing.len(), 1, "only Consequences missing; got {:?}", parsed.findings);
    assert!(missing[0].detail.contains("Consequences"));
}

#[test]
fn bad_slug_is_grammar_finding() {
    let text = "---\nslug: Bad_Slug\nstatus: accepted\n---\n# T\n## Context\n## Decision\n## Consequences\n";
    let parsed = parse_decision(text);
    assert!(parsed.findings.iter().any(|f| f.rule_id == "decision-slug-grammar"));
}

#[test]
fn slug_grammar_rules() {
    assert!(is_valid_slug("a"));
    assert!(is_valid_slug("identity-store-postgres"));
    assert!(is_valid_slug("a1-b2"));
    assert!(!is_valid_slug(""));
    assert!(!is_valid_slug("1abc"));
    assert!(!is_valid_slug("-abc"));
    assert!(!is_valid_slug("Abc"));
    assert!(!is_valid_slug("a_b"));
    assert!(!is_valid_slug(&"a".repeat(SLUG_MAX_LEN + 1)));
}

#[test]
fn heading_allows_trailing_not_deeper() {
    let text = "---\nslug: ok\nstatus: accepted\n---\n# T\n\n## Context (forces)\nx\n\n## Decision\ny\n\n### Consequences\nz\n";
    let parsed = parse_decision(text);
    // `## Context (forces)` satisfies Context; `### Consequences` is a
    // deeper heading and does NOT satisfy Consequences.
    let missing: Vec<_> =
        parsed.findings.iter().filter(|f| f.rule_id == "decision-record-section-missing").collect();
    assert_eq!(missing.len(), 1);
    assert!(missing[0].detail.contains("Consequences"));
}

#[test]
fn baseline_form_round_trips_through_serde() {
    let record = DecisionRecord {
        id: Some("DEC-0007".to_string()),
        slug: "identity-store-postgres".to_string(),
        status: DecisionStatus::Accepted,
        slice: Some("identity-service".to_string()),
        date: Some("2026-06-02".to_string()),
        supersedes: vec!["DEC-0003".to_string()],
        related: vec!["REQ-001".to_string()],
        topics: vec!["identity".to_string()],
        superseded_by: None,
    };
    let yaml = serde_saphyr::to_string(&record).expect("serialise");
    let back: DecisionRecord = serde_saphyr::from_str(&yaml).expect("round-trip");
    assert_eq!(record, back);
    // `superseded-by` uses the kebab rename and stays off the wire when None.
    assert!(!yaml.contains("superseded"));
}
