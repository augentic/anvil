//! Shared imports, helpers, and seeds for the `slice` integration suite.
//!
//! The suite was a single ~1,700-line file; it is now split across the
//! sibling `#[path]` submodules (`create`, `transition`, `touched_specs`,
//! `overlap`, `drop`, `metadata`, `validate`, `provenance`, `model_show`,
//! `validate_file_location`, `validate_catalog`, `synthesize`). Every
//! submodule pulls its shared surface in with `use crate::support::*;`,
//! so the common imports, helpers, and seeds live here once.

pub use std::fs;

pub use crate::common::{Project, parse_json, specify_cmd};

// ---------------------------------------------------------------------------
// Shared seeds
// ---------------------------------------------------------------------------

pub const PLAN_WITH_LEGACY_MONOLITH: &str = "\
name: workflow-prov
lifecycle: pending
sources:
  legacy-monolith:
    adapter: typescript
    path: ./legacy
slices:
  - name: my-slice
    status: pending
    sources:
      - { source: legacy-monolith, lead: my-slice }
";

pub const CLEAN_SPEC_MD: &str = "### Requirement: Password reset request

ID: REQ-001
Sources: [legacy-monolith]
Status: agreed

The system lets a registered user request a password reset link by email.
";

/// Minimal projectable `model.yaml` for a slice named `my-slice` on the
/// earned core (`requirements` + `tasks`), with one fully-projected
/// requirement (kernel-owned fields present), so `slice provenance` can
/// reshape it into the audit view. `value` / `path` are no longer stored
/// here — the projection reads them from `evidence/<source>.yaml`.
pub const CLEAN_MODEL_YAML: &str = "version: 1
slice: my-slice
requirements:
  - id: REQ-001
    title: Password reset request
    status: agreed
    sources: [legacy-monolith]
    claims:
      - source: legacy-monolith
        id: password-reset.request
        kind: requirement
    statement: The system lets a registered user request a password reset link by email.
tasks: []
";

/// Evidence the synthesis kernel resolves authority and anchors claims
/// against. One `requirement` claim, behaviour authority. Shared by
/// the `synthesize` and `plan_dir` suites.
pub const SYNTH_EVIDENCE_YAML: &str = "authority: behaviour
lead: my-slice
claims:
  - id: password-reset.request
    kind: requirement
    statement: \"The system lets a user request a reset link.\"
    path: src/users/reset.ts#L42
";

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

/// Stage a slice with one bound source's Evidence but **no**
/// `plan.yaml` — the workspace-slot shape, where the governing plan
/// lives elsewhere. Callers either seed a project-root plan
/// ([`stage_synthesizable_slice`]-style) or point `--plan-dir` at one.
pub fn stage_synthesizable_slice_without_plan() -> Project {
    let project = Project::init().with_schemas();
    specify_cmd()
        .current_dir(project.root())
        .args(["slice", "create", "my-slice"])
        .assert()
        .success();
    let evidence_dir = project.slices_dir().join("my-slice/evidence");
    fs::create_dir_all(&evidence_dir).expect("mkdir evidence");
    fs::write(evidence_dir.join("legacy-monolith.yaml"), SYNTH_EVIDENCE_YAML)
        .expect("write evidence");
    project
}

/// Stage a slice on disk and seed `<slice>/specs/login/spec.md`
/// directly, plus optionally a `plan.yaml` at the project root, so the
/// provenance gate inside `specify slice validate` has both the spec
/// file and a plan-level source-bindings context to cross-validate
/// against. Returns the project handle so the caller can drive
/// `specify slice validate` on it.
pub fn stage_slice_with_spec(spec_md: &str, plan_yaml: Option<&str>) -> Project {
    let project = Project::init().with_schemas();
    specify_cmd()
        .current_dir(project.root())
        .args(["slice", "create", "my-slice"])
        .assert()
        .success();
    let specs_dir = project.slices_dir().join("my-slice/specs/login");
    fs::create_dir_all(&specs_dir).expect("mkdir specs/login");
    fs::write(specs_dir.join("spec.md"), spec_md).expect("write spec.md");
    if let Some(yaml) = plan_yaml {
        project.seed_plan(yaml);
    }
    project
}

/// Assert the rendered `DiagnosticReport` on stdout carries no finding
/// citing `rule_id`. Tolerates an empty stdout (e.g. a `--dump-model`
/// short-circuit) by treating it as "no findings".
pub fn assert_no_finding(output: &std::process::Output, rule_id: &str) {
    let report: serde_json::Value = match serde_json::from_slice(&output.stdout) {
        Ok(value) => value,
        Err(_) => return,
    };
    if let Some(findings) = report["findings"].as_array() {
        for finding in findings {
            assert_ne!(
                finding["rule-id"], rule_id,
                "no `{rule_id}` finding may appear; got: {findings:#?}"
            );
        }
    }
}

/// Locate the rendered diagnostic on stdout for `rule_id` and return
/// its operator-facing `impact` row. Asserts exit
/// 2 along the way so callers can focus on the impact text.
pub fn find_finding_impact(output: &std::process::Output, rule_id: &str) -> String {
    let err = parse_json(&output.stderr);
    assert_eq!(err["exit-code"], 2);
    let report = parse_json(&output.stdout);
    let findings = report["findings"].as_array().expect("findings array");
    findings
        .iter()
        .find(|r| r["rule-id"] == rule_id)
        .and_then(|r| r["impact"].as_str())
        .unwrap_or_else(|| panic!("`{rule_id}` finding must be present in {findings:#?}"))
        .to_string()
}
