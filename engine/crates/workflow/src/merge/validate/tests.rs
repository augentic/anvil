use super::*;

#[test]
fn design_reference_regex_is_compilable() {
    // If this ever fails, the const changed and the expect() inside
    // `validate_baseline` would start panicking in the wild.
    Regex::new(REQ_ID_PATTERN).unwrap();
}

#[test]
fn clean_baseline_ok() {
    let baseline = "### Requirement: A\n\nID: REQ-001\n\n#### Scenario: x\n\n- ok\n";
    assert!(validate_baseline(baseline).is_empty());
}

#[test]
fn duplicate_ids_fail() {
    let baseline = "### Requirement: A\n\nID: REQ-001\n\n#### Scenario: x\n\n- ok\n\n### Requirement: B\n\nID: REQ-001\n\n#### Scenario: y\n\n- ok\n";
    let results = validate_baseline(baseline);
    let fails: Vec<_> = results.iter().map(as_fail).collect();
    assert!(
        fails.iter().any(|(rid, detail)| *rid == RULE_NO_DUPLICATE_IDS
            && detail.contains("Duplicate ID: REQ-001")),
        "expected duplicate-id fail, got {fails:?}"
    );
}

#[test]
fn missing_id_and_bad_id_fail() {
    let invalid = "### Requirement: A\n\nID: NOT-AN-ID\n\n#### Scenario: x\n\n- ok\n\n### Requirement: B\n\n#### Scenario: y\n\n- ok\n";
    let results = validate_baseline(invalid);
    let fails: Vec<_> = results.iter().map(as_fail).collect();
    assert!(
        fails.iter().any(|(rid, _)| *rid == RULE_ID_MATCHES_PATTERN),
        "expected id-matches-pattern fail, got {fails:?}"
    );
    assert!(
        fails.iter().any(|(rid, _)| *rid == RULE_REQ_HAS_ID),
        "expected requirement-has-id fail, got {fails:?}"
    );
}

fn as_fail(result: &Diagnostic) -> (&str, &str) {
    (result.rule_id.as_deref().unwrap_or(""), result.impact.as_str())
}
