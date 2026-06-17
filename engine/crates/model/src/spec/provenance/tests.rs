//! Unit tests for [`super::parse_spec_md`] + [`super::validate`].

use std::collections::BTreeSet;

use super::*;

macro_rules! fixture {
    ($rel:literal) => {
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/spec/", $rel))
    };
}

fn keys<const N: usize>(items: [&str; N]) -> BTreeSet<String> {
    items.into_iter().map(str::to_string).collect()
}

// ---------------------------------------------------------------------------
// Worked-examples variants
// ---------------------------------------------------------------------------

#[test]
fn parses_single_source_block() {
    let parsed = parse_spec_md(fixture!("single-source.md"));
    assert!(parsed.findings.is_empty(), "no structural findings");
    assert_eq!(parsed.requirements.len(), 1);
    let req = &parsed.requirements[0];
    assert_eq!(req.id, "REQ-001");
    assert_eq!(req.name, "User registration accepts valid email");
    assert_eq!(req.sources, vec!["legacy-monolith"]);
    assert_eq!(req.status, Some(RequirementStatus::Agreed));
    assert_eq!(req.tag, None);
    assert!(req.body.starts_with("The system accepts"));

    let findings = validate(&parsed, &keys(["legacy-monolith"]));
    assert!(findings.is_empty(), "{findings:?}");
}

#[test]
fn parses_combined_agreement_block() {
    let parsed = parse_spec_md(fixture!("combined-agreement.md"));
    assert!(parsed.findings.is_empty());
    let req = &parsed.requirements[0];
    assert_eq!(req.sources, vec!["identity-design-notes", "legacy-monolith"]);
    assert_eq!(req.status, Some(RequirementStatus::Agreed));
    assert_eq!(req.tag, None);

    let findings = validate(&parsed, &keys(["identity-design-notes", "legacy-monolith"]));
    assert!(findings.is_empty(), "{findings:?}");
}

#[test]
fn parses_divergence_block_with_inline_tag() {
    let parsed = parse_spec_md(fixture!("divergence.md"));
    assert!(parsed.findings.is_empty());
    let req = &parsed.requirements[0];
    assert_eq!(req.name, "Reset link expiry");
    assert_eq!(req.tag, Some(RequirementTag::Divergence));
    assert_eq!(req.status, Some(RequirementStatus::Divergence));
    assert_eq!(req.sources, vec!["identity-design-notes", "legacy-monolith"]);

    let findings = validate(&parsed, &keys(["identity-design-notes", "legacy-monolith"]));
    assert!(findings.is_empty(), "{findings:?}");
}

#[test]
fn parses_conflict_block_with_inline_tag() {
    let parsed = parse_spec_md(fixture!("conflict.md"));
    assert!(parsed.findings.is_empty());
    let req = &parsed.requirements[0];
    assert_eq!(req.tag, Some(RequirementTag::Conflict));
    assert_eq!(req.status, Some(RequirementStatus::Conflict));

    let findings = validate(&parsed, &keys(["docs-a", "docs-b"]));
    assert!(findings.is_empty(), "{findings:?}");
}

#[test]
fn parses_unknown_block_with_inline_tag() {
    let parsed = parse_spec_md(fixture!("unknown.md"));
    assert!(parsed.findings.is_empty());
    let req = &parsed.requirements[0];
    assert_eq!(req.tag, Some(RequirementTag::Unknown));
    assert_eq!(req.status, Some(RequirementStatus::Unknown));
    assert_eq!(req.sources, vec!["intent"]);

    let findings = validate(&parsed, &keys(["intent"]));
    assert!(findings.is_empty(), "{findings:?}");
}

#[test]
fn parses_multi_block_document() {
    let parsed = parse_spec_md(fixture!("multi-block.md"));
    assert!(parsed.findings.is_empty(), "{:?}", parsed.findings);
    assert_eq!(parsed.requirements.len(), 2);
    let ids: Vec<&str> = parsed.requirements.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(ids, vec!["REQ-001", "REQ-002"]);
    assert_eq!(parsed.requirements[0].status, Some(RequirementStatus::Agreed));
    assert_eq!(parsed.requirements[1].status, Some(RequirementStatus::Divergence));
    assert_eq!(parsed.requirements[1].tag, Some(RequirementTag::Divergence));
}

// ---------------------------------------------------------------------------
// Validation failure modes
// ---------------------------------------------------------------------------

#[test]
fn missing_id_reported() {
    let parsed =
        parse_spec_md("### Requirement: Missing id\n\nSources: [a]\nStatus: agreed\n\nbody\n");
    let findings = validate(&parsed, &keys(["a"]));
    assert!(findings.iter().any(|f| f.rule_id == "spec.requirement-id-missing"), "{findings:?}");
}

#[test]
fn malformed_id_reported() {
    let parsed =
        parse_spec_md("### Requirement: Bad id\n\nID: REQ-1\nSources: [a]\nStatus: agreed\n");
    let findings = validate(&parsed, &keys(["a"]));
    assert!(findings.iter().any(|f| f.rule_id == "spec.requirement-id-malformed"), "{findings:?}");
}

#[test]
fn missing_sources_reported() {
    let parsed = parse_spec_md("### Requirement: No sources\n\nID: REQ-001\nStatus: agreed\n");
    let findings = validate(&parsed, &keys(["a"]));
    assert!(
        findings.iter().any(|f| f.rule_id == "spec.requirement-sources-missing"),
        "{findings:?}"
    );
}

#[test]
fn empty_sources_reported() {
    let parsed = parse_spec_md(
        "### Requirement: Empty sources\n\nID: REQ-001\nSources: []\nStatus: agreed\n",
    );
    let findings = validate(&parsed, &keys(["a"]));
    assert!(findings.iter().any(|f| f.rule_id == "spec.requirement-sources-empty"), "{findings:?}");
}

#[test]
fn empty_sources_legal_for_unknown() {
    // Contract: `Sources: []` appears exactly when `Status: unknown` —
    // an evidence-less requirement (e.g. on a reconciliation-inserted
    // bootstrap slice) has no contributing source to cite.
    let parsed = parse_spec_md(
        "### Requirement: Evidence-less [unknown]\n\nID: REQ-001\nSources: []\nStatus: unknown\n",
    );
    let findings = validate(&parsed, &keys(["a"]));
    assert!(
        !findings.iter().any(|f| f.rule_id == "spec.requirement-sources-empty"),
        "{findings:?}"
    );
}

#[test]
fn missing_status_reported() {
    let parsed = parse_spec_md("### Requirement: No status\n\nID: REQ-001\nSources: [a]\n");
    let findings = validate(&parsed, &keys(["a"]));
    assert!(
        findings.iter().any(|f| f.rule_id == "spec.requirement-status-missing"),
        "{findings:?}"
    );
}

#[test]
fn unknown_status_value_reported() {
    let parsed = parse_spec_md(
        "### Requirement: Bogus status\n\nID: REQ-001\nSources: [a]\nStatus: maybe\n",
    );
    let findings = validate(&parsed, &keys(["a"]));
    assert!(
        findings.iter().any(|f| f.rule_id == "spec.requirement-status-unknown-value"),
        "{findings:?}"
    );
    assert_eq!(parsed.requirements[0].status_raw.as_deref(), Some("maybe"));
    assert_eq!(parsed.requirements[0].status, None);
}

#[test]
fn source_key_undefined_reported() {
    let parsed = parse_spec_md(
        "### Requirement: Unknown source key\n\nID: REQ-001\nSources: [phantom]\nStatus: agreed\n",
    );
    let findings = validate(&parsed, &keys(["a", "b"]));
    assert!(
        findings.iter().any(|f| f.rule_id == "spec.requirement-source-undefined"),
        "{findings:?}"
    );
}

#[test]
fn malformed_source_key_reported() {
    let parsed = parse_spec_md(
        "### Requirement: Bad key\n\nID: REQ-001\nSources: [Not_Kebab]\nStatus: agreed\n",
    );
    let findings = validate(&parsed, &BTreeSet::new());
    assert!(
        findings.iter().any(|f| f.rule_id == "spec.requirement-source-malformed"),
        "{findings:?}"
    );
}

#[test]
fn tag_mismatch_when_tag_lies() {
    let parsed = parse_spec_md(
        "### Requirement: Mismatched tag [divergence]\n\nID: REQ-001\nSources: [a]\nStatus: agreed\n",
    );
    let findings = validate(&parsed, &keys(["a"]));
    assert!(
        findings.iter().any(|f| f.rule_id == "spec.requirement-tag-status-mismatch"),
        "{findings:?}"
    );
}

#[test]
fn tag_mismatch_when_status_lies() {
    let parsed = parse_spec_md(
        "### Requirement: Status without tag\n\nID: REQ-001\nSources: [a]\nStatus: divergence\n",
    );
    let findings = validate(&parsed, &keys(["a"]));
    assert!(
        findings.iter().any(|f| f.rule_id == "spec.requirement-tag-status-mismatch"),
        "{findings:?}"
    );
}

// ---------------------------------------------------------------------------
// Liberal / metadata-free behaviours
// ---------------------------------------------------------------------------

#[test]
fn unannotated_file_is_skipped() {
    let parsed = parse_spec_md(fixture!("unannotated-legacy.md"));
    assert!(parsed.is_unannotated());
    assert_eq!(parsed.requirements.len(), 1);
}

#[test]
fn empty_input_parses_to_empty_spec() {
    let parsed = parse_spec_md("");
    assert!(parsed.requirements.is_empty());
    assert!(parsed.findings.is_empty());
    assert!(parsed.is_unannotated());
}

#[test]
fn liberal_brackets_in_sources_line() {
    let bare = parse_spec_md(
        "### Requirement: Bare sources\n\nID: REQ-001\nSources: a, b, c\nStatus: agreed\n",
    );
    assert_eq!(bare.requirements[0].sources, vec!["a", "b", "c"]);
    let bracketed = parse_spec_md(
        "### Requirement: Bracketed sources\n\nID: REQ-001\nSources: [a, b, c]\nStatus: agreed\n",
    );
    assert_eq!(bracketed.requirements[0].sources, vec!["a", "b", "c"]);
}

#[test]
fn body_preserves_interior_blank_lines() {
    let parsed = parse_spec_md(
        "### Requirement: Multi-paragraph body\n\nID: REQ-001\nSources: [a]\nStatus: agreed\n\nFirst paragraph.\n\nSecond paragraph.\n",
    );
    let body = &parsed.requirements[0].body;
    assert!(body.contains("First paragraph."));
    assert!(body.contains("Second paragraph."));
    assert!(body.contains("\n\n"), "interior blank line preserved");
}

#[test]
fn into_diagnostic_prefixes_path_hint() {
    let parsed = parse_spec_md("### Requirement: No id\n\nSources: [a]\nStatus: agreed\n");
    let mut findings = validate(&parsed, &keys(["a"]));
    let diagnostic =
        findings.pop().expect("at least one finding").into_diagnostic("specs/login/spec.md");
    assert!(diagnostic.impact.starts_with("specs/login/spec.md:"), "{}", diagnostic.impact);
    assert_eq!(diagnostic.location.as_ref().map(|l| l.path.as_str()), Some("specs/login/spec.md"));
}

// ---------------------------------------------------------------------------
// Source-key + req-id shape predicates
// ---------------------------------------------------------------------------

#[test]
fn source_key_shape_predicate() {
    assert!(is_valid_source_key("a"));
    assert!(is_valid_source_key("legacy-monolith"));
    assert!(is_valid_source_key("a1-b2"));
    assert!(!is_valid_source_key(""));
    assert!(!is_valid_source_key("1abc"));
    assert!(!is_valid_source_key("Abc"));
    assert!(!is_valid_source_key("a--b"));
    assert!(!is_valid_source_key("a-"));
    assert!(!is_valid_source_key("a_b"));
}

#[test]
fn req_id_shape_predicate() {
    assert!(is_valid_req_id("REQ-001"));
    assert!(is_valid_req_id("REQ-999"));
    assert!(!is_valid_req_id("REQ-1"));
    assert!(!is_valid_req_id("REQ-1234"));
    assert!(!is_valid_req_id("req-001"));
    assert!(!is_valid_req_id("REQ-00A"));
    assert!(!is_valid_req_id(""));
}

#[test]
fn requirement_status_round_trips() {
    for (variant, wire) in [
        (RequirementStatus::Agreed, "agreed"),
        (RequirementStatus::Unknown, "unknown"),
        (RequirementStatus::Conflict, "conflict"),
        (RequirementStatus::Divergence, "divergence"),
    ] {
        assert_eq!(serde_json::to_string(&variant).expect("serialise"), format!("\"{wire}\""));
    }
}
