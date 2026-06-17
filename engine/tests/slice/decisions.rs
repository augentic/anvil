//! Integration tests for the Decision Record gate in
//! `specify slice validate` — the five `decision-*` findings over
//! `<slice>/decisions/*.md`.
//!
//! Each test crafts a slice that trips exactly one finding and asserts
//! it fires (blocking, exit 2); the clean slice asserts none fire. Test
//! style follows `tests/slice_drift.rs`: drive the built binary and
//! inspect the rendered `DiagnosticReport` on stdout.

use std::fs;

use crate::common::{Project, parse_json, specify_cmd};

/// Body of a well-formed slice-authored Decision Record.
fn record(slug: &str, status: &str, supersedes: &[&str]) -> String {
    let sup = if supersedes.is_empty() {
        String::new()
    } else {
        format!("supersedes: [{}]\n", supersedes.join(", "))
    };
    format!(
        "---\nslug: {slug}\nstatus: {status}\n{sup}---\n# Title for {slug}\n\n\
         ## Context\nWhy.\n\n## Decision\nWhat.\n\n## Consequences\nTrade-offs.\n"
    )
}

/// Stage `my-slice` with the given `decisions/<file>` entries and
/// optional promoted baseline records under `.specify/decisions/`.
fn stage(decisions: &[(&str, String)], baseline: &[(&str, String)]) -> Project {
    let project = Project::init().with_schemas();
    specify_cmd()
        .current_dir(project.root())
        .args(["slice", "create", "my-slice"])
        .assert()
        .success();

    let dir = project.slices_dir().join("my-slice").join("decisions");
    fs::create_dir_all(&dir).expect("mkdir slice decisions");
    for (file, body) in decisions {
        fs::write(dir.join(file), body).expect("write slice decision");
    }

    if !baseline.is_empty() {
        let base = project.root().join(".specify/decisions");
        fs::create_dir_all(&base).expect("mkdir baseline decisions");
        for (file, body) in baseline {
            fs::write(base.join(file), body).expect("write baseline decision");
        }
    }
    project
}

fn validate(project: &Project) -> std::process::Output {
    specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "slice", "validate", "my-slice"])
        .assert()
        .get_output()
        .clone()
}

fn assert_fires(output: &std::process::Output, rule_id: &str) {
    assert_eq!(output.status.code(), Some(2), "decision findings must gate exit 2");
    let report = parse_json(&output.stdout);
    let findings = report["findings"].as_array().expect("findings array");
    assert!(
        findings.iter().any(|f| f["rule-id"] == rule_id),
        "expected finding `{rule_id}` in: {findings:#?}"
    );
}

fn assert_silent(output: &std::process::Output, rule_id: &str) {
    let Ok(report) = serde_json::from_slice::<serde_json::Value>(&output.stdout) else {
        return;
    };
    if let Some(findings) = report["findings"].as_array() {
        for finding in findings {
            assert_ne!(finding["rule-id"], rule_id, "`{rule_id}` must not fire: {findings:#?}");
        }
    }
}

#[test]
fn clean_record_raises_no_decision_findings() {
    let project = stage(&[("use-postgres.md", record("use-postgres", "accepted", &[]))], &[]);
    let output = validate(&project);
    for rule in [
        "decision-record-schema",
        "decision-record-section-missing",
        "decision-slug-grammar",
        "decision-slug-collision",
        "decision-supersede-orphan",
    ] {
        assert_silent(&output, rule);
    }
}

#[test]
fn missing_section_fires() {
    let body = "---\nslug: ok\nstatus: accepted\n---\n# T\n\n## Context\nc\n\n## Decision\nd\n";
    let project = stage(&[("ok.md", body.to_string())], &[]);
    assert_fires(&validate(&project), "decision-record-section-missing");
}

#[test]
fn bad_slug_grammar_fires() {
    let body = record("Bad_Slug", "accepted", &[]);
    let project = stage(&[("bad.md", body)], &[]);
    assert_fires(&validate(&project), "decision-slug-grammar");
}

#[test]
fn bad_schema_fires() {
    // `status: maybe` is not in the closed enum.
    let body = "---\nslug: ok\nstatus: maybe\n---\n# T\n\n## Context\nc\n\n## Decision\nd\n\n## Consequences\ne\n";
    let project = stage(&[("ok.md", body.to_string())], &[]);
    assert_fires(&validate(&project), "decision-record-schema");
}

#[test]
fn slug_collision_fires() {
    let project = stage(
        &[("a.md", record("dup", "accepted", &[])), ("b.md", record("dup", "rejected", &[]))],
        &[],
    );
    assert_fires(&validate(&project), "decision-slug-collision");
}

#[test]
fn supersede_orphan_fires() {
    let project = stage(&[("new.md", record("new-store", "accepted", &["DEC-9999"]))], &[]);
    assert_fires(&validate(&project), "decision-supersede-orphan");
}

#[test]
fn supersede_to_baseline_silent() {
    let baseline = "---\nid: DEC-0001\nslug: old-store\nstatus: accepted\nslice: s\ndate: 2026-06-02\n---\n# Old\n\n## Context\nc\n\n## Decision\nd\n\n## Consequences\ne\n";
    let project = stage(
        &[("new.md", record("new-store", "accepted", &["DEC-0001"]))],
        &[("DEC-0001-old-store.md", baseline.to_string())],
    );
    assert_silent(&validate(&project), "decision-supersede-orphan");
}

#[test]
fn supersede_to_sibling_silent() {
    let project = stage(
        &[
            ("alpha.md", record("alpha", "accepted", &[])),
            ("beta.md", record("beta", "accepted", &["alpha"])),
        ],
        &[],
    );
    assert_silent(&validate(&project), "decision-supersede-orphan");
}
