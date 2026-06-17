//! Global `--plan-dir` plan-root override (env `SPECIFY_PLAN_DIR`).
//!
//! Workspace routing runs phase verbs inside a materialised slot while
//! the governing `plan.yaml` stays at the initiating workspace root —
//! by design no slot grows its own plan. These tests pin the bridge:
//! slice-time plan readers resolve the plan against the override
//! instead of the project root, and a wrong override fails with the
//! same typed error citing the overridden path.

use tempfile::tempdir;

use crate::support::*;

/// Stand up a plan-less slot project plus a sibling "workspace" dir
/// holding the governing `plan.yaml`.
fn stage_slot_and_workspace() -> (Project, tempfile::TempDir) {
    let project = stage_synthesizable_slice_without_plan();
    let workspace = tempdir().expect("workspace tempdir");
    fs::write(workspace.path().join("plan.yaml"), PLAN_WITH_LEGACY_MONOLITH)
        .expect("write workspace plan.yaml");
    (project, workspace)
}

#[test]
fn synthesize_resolves_plan_via_flag() {
    let (project, workspace) = stage_slot_and_workspace();
    assert!(!project.plan_path().exists(), "the slot must carry no plan.yaml");

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "--plan-dir"])
        .arg(workspace.path())
        .args(["slice", "synthesize", "my-slice", "--dry-run"])
        .assert()
        .success();

    let value = parse_json(&assert.get_output().stdout);
    assert_eq!(value["kind"], "inputs");
    assert_eq!(value["slice"], "my-slice");
}

#[test]
fn synthesize_resolves_plan_via_env() {
    let (project, workspace) = stage_slot_and_workspace();

    specify_cmd()
        .current_dir(project.root())
        .env("SPECIFY_PLAN_DIR", workspace.path())
        .args(["--format", "json", "slice", "synthesize", "my-slice", "--dry-run"])
        .assert()
        .success();
}

#[test]
fn synthesize_errors_cite_override_path() {
    // An override pointing at a plan-less directory keeps the typed
    // plan-missing error, and the message names the overridden path so
    // a mis-wired executor is diagnosable from the envelope alone.
    let (project, _workspace) = stage_slot_and_workspace();
    let empty = tempdir().expect("empty tempdir");

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "--plan-dir"])
        .arg(empty.path())
        .args(["slice", "synthesize", "my-slice", "--dry-run"])
        .assert()
        .failure();

    assert_eq!(assert.get_output().status.code(), Some(2));
    let value = parse_json(&assert.get_output().stderr);
    assert_eq!(value["error"], "slice-synthesize-plan-missing");
    let message = value["message"].as_str().expect("message string");
    assert!(
        message.contains(empty.path().to_str().expect("utf8 tempdir")),
        "message must cite the overridden plan path, got: {message}"
    );
}

/// A clean, fully-projected `model.yaml` whose `project: test-proj`
/// must agree with the plan entry's `project` — the plan-dependent
/// `slice-model-target-drift` gate is the probe that proves which
/// plan `slice validate` consulted.
const PLAN_DIR_MODEL: &str = "version: 1
slice: my-slice
project: test-proj
requirements:
  - id: REQ-001
    title: Password reset request
    status: agreed
    domain: password-reset
    sources: [legacy-monolith]
    claims:
      - source: legacy-monolith
        id: password-reset.request
        kind: requirement
    statement: The system lets a registered user request a password reset link by email.
tasks:
  - id: TASK-001
    text: Implement password reset request handling.
    satisfies: [REQ-001]
";

/// Workspace plan whose `my-slice` entry binds `project: <project>`.
fn plan_binding_project(project: &str) -> String {
    format!(
        "\
name: plan-dir
lifecycle: pending
sources:
  legacy-monolith:
    adapter: typescript
    path: ./legacy
slices:
  - name: my-slice
    status: pending
    project: {project}
    sources:
      - {{ source: legacy-monolith, lead: my-slice }}
"
    )
}

/// Stage the slot slice (model + spec on top of the shared Evidence
/// staging), write `plan_yaml` into a sibling workspace dir, and run
/// `slice validate --plan-dir <workspace>`, returning the output.
fn validate_with_workspace_plan(plan_yaml: &str) -> std::process::Output {
    let project = stage_synthesizable_slice_without_plan();
    let slice_dir = project.slices_dir().join("my-slice");
    fs::write(slice_dir.join("model.yaml"), PLAN_DIR_MODEL).expect("write model.yaml");
    let spec_dir = slice_dir.join("specs/password-reset");
    fs::create_dir_all(&spec_dir).expect("mkdir specs/password-reset");
    fs::write(
        spec_dir.join("spec.md"),
        "### Requirement: Password reset request

ID: REQ-001
Sources: legacy-monolith
Status: agreed

The system lets a registered user request a password reset link by email.
",
    )
    .expect("write spec.md");

    let workspace = tempdir().expect("workspace tempdir");
    fs::write(workspace.path().join("plan.yaml"), plan_yaml).expect("write workspace plan.yaml");

    specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "--plan-dir"])
        .arg(workspace.path())
        .args(["slice", "validate", "my-slice"])
        .assert()
        .get_output()
        .clone()
}

#[test]
fn validate_reads_plan_via_flag() {
    // Agreeing projects: the plan-dependent target-drift gate stays
    // silent, proving the override plan satisfied the cross-check.
    let output = validate_with_workspace_plan(&plan_binding_project("test-proj"));
    assert_no_finding(&output, "slice-model-target-drift");

    // Disagreeing projects: the same gate fires — the workspace plan,
    // not a (non-existent) slot plan, is the one consulted.
    let output = validate_with_workspace_plan(&plan_binding_project("beta"));
    assert_eq!(output.status.code(), Some(2));
    let report = parse_json(&output.stdout);
    let findings = report["findings"].as_array().expect("findings array");
    assert!(
        findings.iter().any(|f| f["rule-id"] == "slice-model-target-drift"),
        "target-drift must fire against the override plan, got: {findings:#?}"
    );
}
