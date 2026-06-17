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

/// Worked-examples: each annotated single-requirement fixture parses to one
/// requirement with the expected sources/status/tag and validates cleanly
/// against its declared source keys. `multi-block.md` is asserted separately.
#[test]
fn parses_worked_example_blocks() {
    struct Case {
        body: &'static str,
        // Asserted exactly when `Some`; the conflict fixture only pins its
        // validate keys, mirroring the original per-block coverage.
        sources: Option<&'static [&'static str]>,
        status: RequirementStatus,
        tag: Option<RequirementTag>,
        keys: &'static [&'static str],
    }
    let cases = [
        Case {
            body: fixture!("single-source.md"),
            sources: Some(&["legacy-monolith"]),
            status: RequirementStatus::Agreed,
            tag: None,
            keys: &["legacy-monolith"],
        },
        Case {
            body: fixture!("combined-agreement.md"),
            sources: Some(&["identity-design-notes", "legacy-monolith"]),
            status: RequirementStatus::Agreed,
            tag: None,
            keys: &["identity-design-notes", "legacy-monolith"],
        },
        Case {
            body: fixture!("divergence.md"),
            sources: Some(&["identity-design-notes", "legacy-monolith"]),
            status: RequirementStatus::Divergence,
            tag: Some(RequirementTag::Divergence),
            keys: &["identity-design-notes", "legacy-monolith"],
        },
        Case {
            body: fixture!("conflict.md"),
            sources: None,
            status: RequirementStatus::Conflict,
            tag: Some(RequirementTag::Conflict),
            keys: &["docs-a", "docs-b"],
        },
        Case {
            body: fixture!("unknown.md"),
            sources: Some(&["intent"]),
            status: RequirementStatus::Unknown,
            tag: Some(RequirementTag::Unknown),
            keys: &["intent"],
        },
    ];

    for case in cases {
        let parsed = parse_spec_md(case.body);
        assert!(parsed.findings.is_empty(), "structural findings: {:?}", parsed.findings);
        assert_eq!(parsed.requirements.len(), 1);
        let req = &parsed.requirements[0];
        if let Some(expected) = case.sources {
            let sources: Vec<&str> = req.sources.iter().map(String::as_str).collect();
            assert_eq!(sources, expected);
        }
        assert_eq!(req.status, Some(case.status));
        assert_eq!(req.tag, case.tag);

        let key_set: BTreeSet<String> = case.keys.iter().map(|s| (*s).to_string()).collect();
        let findings = validate(&parsed, &key_set);
        assert!(findings.is_empty(), "{findings:?}");
    }
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

/// Validation failure modes: each malformed requirement block surfaces its
/// specific `spec.requirement-*` finding. One row per distinct rule id.
#[test]
fn validation_failure_modes() {
    let cases: &[(&str, &[&str], &str)] = &[
        (
            "### Requirement: Missing id\n\nSources: [a]\nStatus: agreed\n\nbody\n",
            &["a"],
            "spec.requirement-id-missing",
        ),
        (
            "### Requirement: Bad id\n\nID: REQ-1\nSources: [a]\nStatus: agreed\n",
            &["a"],
            "spec.requirement-id-malformed",
        ),
        (
            "### Requirement: No sources\n\nID: REQ-001\nStatus: agreed\n",
            &["a"],
            "spec.requirement-sources-missing",
        ),
        (
            "### Requirement: Empty sources\n\nID: REQ-001\nSources: []\nStatus: agreed\n",
            &["a"],
            "spec.requirement-sources-empty",
        ),
        (
            "### Requirement: No status\n\nID: REQ-001\nSources: [a]\n",
            &["a"],
            "spec.requirement-status-missing",
        ),
        (
            "### Requirement: Bogus status\n\nID: REQ-001\nSources: [a]\nStatus: maybe\n",
            &["a"],
            "spec.requirement-status-unknown-value",
        ),
        (
            "### Requirement: Unknown source key\n\nID: REQ-001\nSources: [phantom]\nStatus: agreed\n",
            &["a", "b"],
            "spec.requirement-source-undefined",
        ),
        (
            "### Requirement: Bad key\n\nID: REQ-001\nSources: [Not_Kebab]\nStatus: agreed\n",
            &[],
            "spec.requirement-source-malformed",
        ),
        (
            "### Requirement: Mismatched tag [divergence]\n\nID: REQ-001\nSources: [a]\nStatus: agreed\n",
            &["a"],
            "spec.requirement-tag-status-mismatch",
        ),
        (
            "### Requirement: Status without tag\n\nID: REQ-001\nSources: [a]\nStatus: divergence\n",
            &["a"],
            "spec.requirement-tag-status-mismatch",
        ),
    ];
    for (md, key_list, rule_id) in cases {
        let parsed = parse_spec_md(md);
        let key_set: BTreeSet<String> = key_list.iter().map(|s| (*s).to_string()).collect();
        let findings = validate(&parsed, &key_set);
        assert!(findings.iter().any(|f| f.rule_id == *rule_id), "{rule_id}: {findings:?}");
    }

    // `Status: maybe` retains the raw token and leaves the typed status unset.
    let parsed = parse_spec_md(
        "### Requirement: Bogus status\n\nID: REQ-001\nSources: [a]\nStatus: maybe\n",
    );
    assert_eq!(parsed.requirements[0].status_raw.as_deref(), Some("maybe"));
    assert_eq!(parsed.requirements[0].status, None);
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
