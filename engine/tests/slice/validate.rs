//! `slice validate` gates: the workflow §Requirement-block provenance
//! contract, the metadata-free provenance skip, and the
//! `discovery-lead-synopsis-thin` advisory.

use crate::support::*;

/// The validate surface now renders a `DiagnosticReport` on stdout and
/// fails payload-free: the per-rule discriminant lives in
/// `findings[].rule-id` on stdout, while stderr carries only the
/// payload-free `Error::Validation` envelope (exit 2). Assert the
/// expected `rule_id` appears in the rendered findings exactly.
fn assert_provenance_fail_rule(output: &std::process::Output, rule_id: &str) {
    let err = parse_json(&output.stderr);
    assert_eq!(err["exit-code"], 2);
    let report = parse_json(&output.stdout);
    let findings = report["findings"].as_array().expect("findings array");
    assert!(
        findings.iter().any(|r| r["rule-id"] == rule_id),
        "expected rule_id `{rule_id}` in findings: {findings:#?}"
    );
}

#[test]
fn validate_rejects_missing_id() {
    let spec = "### Requirement: Missing id\n\n\
                Sources: [legacy-monolith]\n\
                Status: agreed\n\n\
                body\n";
    let project = stage_slice_with_spec(spec, Some(PLAN_WITH_LEGACY_MONOLITH));
    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "slice", "validate", "my-slice"])
        .assert()
        .failure();
    assert_eq!(assert.get_output().status.code(), Some(2));
    assert_provenance_fail_rule(assert.get_output(), "spec.requirement-id-missing");
}

#[test]
fn validate_rejects_malformed_id() {
    let spec = "### Requirement: Malformed id\n\n\
                ID: REQ-1\n\
                Sources: [legacy-monolith]\n\
                Status: agreed\n";
    let project = stage_slice_with_spec(spec, Some(PLAN_WITH_LEGACY_MONOLITH));
    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "slice", "validate", "my-slice"])
        .assert()
        .failure();
    assert_eq!(assert.get_output().status.code(), Some(2));
    assert_provenance_fail_rule(assert.get_output(), "spec.requirement-id-malformed");
}

#[test]
fn validate_rejects_missing_sources() {
    let spec = "### Requirement: No sources\n\n\
                ID: REQ-001\n\
                Status: agreed\n";
    let project = stage_slice_with_spec(spec, Some(PLAN_WITH_LEGACY_MONOLITH));
    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "slice", "validate", "my-slice"])
        .assert()
        .failure();
    assert_eq!(assert.get_output().status.code(), Some(2));
    assert_provenance_fail_rule(assert.get_output(), "spec.requirement-sources-missing");
}

#[test]
fn validate_rejects_missing_status() {
    let spec = "### Requirement: No status\n\n\
                ID: REQ-001\n\
                Sources: [legacy-monolith]\n";
    let project = stage_slice_with_spec(spec, Some(PLAN_WITH_LEGACY_MONOLITH));
    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "slice", "validate", "my-slice"])
        .assert()
        .failure();
    assert_eq!(assert.get_output().status.code(), Some(2));
    assert_provenance_fail_rule(assert.get_output(), "spec.requirement-status-missing");
}

#[test]
fn validate_rejects_unknown_status() {
    let spec = "### Requirement: Bogus status\n\n\
                ID: REQ-001\n\
                Sources: [legacy-monolith]\n\
                Status: maybe\n";
    let project = stage_slice_with_spec(spec, Some(PLAN_WITH_LEGACY_MONOLITH));
    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "slice", "validate", "my-slice"])
        .assert()
        .failure();
    assert_eq!(assert.get_output().status.code(), Some(2));
    assert_provenance_fail_rule(assert.get_output(), "spec.requirement-status-unknown-value");
}

#[test]
fn validate_rejects_source_not_in_plan() {
    let spec = "### Requirement: Stray source key\n\n\
                ID: REQ-001\n\
                Sources: [phantom]\n\
                Status: agreed\n";
    let project = stage_slice_with_spec(spec, Some(PLAN_WITH_LEGACY_MONOLITH));
    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "slice", "validate", "my-slice"])
        .assert()
        .failure();
    assert_eq!(assert.get_output().status.code(), Some(2));
    assert_provenance_fail_rule(assert.get_output(), "spec.requirement-source-undefined");
}

#[test]
fn validate_rejects_tag_status_mismatch() {
    let spec = "### Requirement: Lying tag [divergence]\n\n\
                ID: REQ-001\n\
                Sources: [legacy-monolith]\n\
                Status: agreed\n";
    let project = stage_slice_with_spec(spec, Some(PLAN_WITH_LEGACY_MONOLITH));
    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "slice", "validate", "my-slice"])
        .assert()
        .failure();
    assert_eq!(assert.get_output().status.code(), Some(2));
    assert_provenance_fail_rule(assert.get_output(), "spec.requirement-tag-status-mismatch");
}

#[test]
fn skips_provenance_no_metadata() {
    // Metadata-free (pre-synthesis) state. The provenance gate must
    // not fire and the slice progresses to the existing adapter rule
    // run. The adapter rules will still surface deferred /
    // pass-style results — we only assert the provenance rule ids
    // are NOT present.
    let spec = "### Requirement: metadata-free body\n\n\
                ID: REQ-001\n\n\
                body that has no Sources or Status yet\n";
    let project = stage_slice_with_spec(spec, None);
    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "slice", "validate", "my-slice"])
        .assert();
    // Whether the run passes or fails (existing adapter rules may
    // still produce findings on the synthetic slice), no provenance
    // rule should appear on the rendered report.
    if let Ok(report) = serde_json::from_slice::<serde_json::Value>(&assert.get_output().stdout)
        && let Some(findings) = report["findings"].as_array()
    {
        for finding in findings {
            let rule_id = finding["rule-id"].as_str().unwrap_or("");
            assert!(
                !rule_id.starts_with("spec.requirement-"),
                "no provenance rule should fire on a metadata-free spec.md, got: {rule_id}"
            );
        }
    }
}

#[test]
fn flags_thin_synopsis_non_blocking() {
    // A thin same-slug synopsis the agent cannot match or split on,
    // alongside a content-bearing one. The advisory must surface at
    // `suggestion` severity (non-blocking by the shared
    // `blocking_present` predicate — only `critical`/`important`
    // violations gate exit), nudging without parking the slice. Only
    // the thin `docs:identity-api` lead is flagged; the content-bearing
    // `legacy:identity-api` lead is not. (Adapter validation still
    // surfaces unrelated findings on this synthetic slice, so the test
    // asserts on the advisory finding itself rather than the overall
    // exit code — matching the suite's `assert_no_finding` convention.)
    let project = Project::init();
    specify_cmd()
        .current_dir(project.root())
        .args(["slice", "create", "my-slice"])
        .assert()
        .success();

    let discovery = "\
# Discovery — identity

## Lead inventory

### docs:identity-api

- lead: identity-api
- source: docs
- synopsis: Identity API.

### legacy:identity-api

- lead: identity-api
- source: legacy
- synopsis: Authentication and account-access API covering login, token refresh, and profile reads.
";
    fs::write(project.root().join("discovery.md"), discovery).expect("write discovery.md");

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "slice", "validate", "my-slice"])
        .assert();
    let report = parse_json(&assert.get_output().stdout);
    let findings = report["findings"].as_array().expect("findings array");
    let thin: Vec<_> =
        findings.iter().filter(|f| f["rule-id"] == "discovery-lead-synopsis-thin").collect();
    assert_eq!(
        thin.len(),
        1,
        "exactly one thin-synopsis finding expected (only the `docs:identity-api` lead), got: \
         {findings:#?}"
    );
    let impact = thin[0]["impact"].as_str().unwrap_or_default();
    assert!(impact.contains("docs:identity-api"), "finding must name the thin lead, got: {impact}");
    let severity = thin[0]["severity"].as_str().unwrap_or_default();
    assert_eq!(
        severity, "suggestion",
        "advisory finding must be `suggestion` severity so it never blocks"
    );
}
