//! Provenance projection — `slice provenance`.

use crate::support::*;

/// Evidence the provenance projection reads `value` / `path` and
/// document-level `authority` from when reshaping `CLEAN_MODEL_YAML`.
const CLEAN_EVIDENCE_YAML: &str = "authority: behaviour
lead: my-slice
claims:
  - id: password-reset.request
    kind: requirement
    statement: \"Password reset request returns a 200 response.\"
    path: src/users/reset.ts#L42
";

#[test]
fn provenance_projects_from_model() {
    let project = stage_slice_with_spec(CLEAN_SPEC_MD, Some(PLAN_WITH_LEGACY_MONOLITH));
    let slice_dir = project.slices_dir().join("my-slice");
    fs::write(slice_dir.join("model.yaml"), CLEAN_MODEL_YAML).expect("write model.yaml");
    let evidence_dir = slice_dir.join("evidence");
    fs::create_dir_all(&evidence_dir).expect("mkdir evidence");
    fs::write(evidence_dir.join("legacy-monolith.yaml"), CLEAN_EVIDENCE_YAML)
        .expect("write evidence");
    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "slice", "provenance", "my-slice"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(stdout.contains("REQ-001"), "projection should list REQ-001, got: {stdout}");
    assert!(
        stdout.contains("single-source"),
        "projection should carry the resolution, got: {stdout}"
    );
}

#[test]
fn provenance_fails_without_model() {
    let project = stage_slice_with_spec(CLEAN_SPEC_MD, Some(PLAN_WITH_LEGACY_MONOLITH));
    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "slice", "provenance", "my-slice"])
        .assert()
        .failure();
    assert_eq!(assert.get_output().status.code(), Some(2));
}
