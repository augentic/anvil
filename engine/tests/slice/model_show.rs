//! Model viewer — `slice model show`.

use crate::support::*;

#[test]
fn model_show_renders_json_and_text() {
    let project = stage_slice_with_spec(CLEAN_SPEC_MD, Some(PLAN_WITH_LEGACY_MONOLITH));
    let slice_dir = project.slices_dir().join("my-slice");
    fs::write(slice_dir.join("model.yaml"), CLEAN_MODEL_YAML).expect("write model.yaml");

    // `--format json` serialises the persisted model verbatim.
    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "slice", "model", "show", "my-slice"])
        .assert()
        .success();
    let value = parse_json(&assert.get_output().stdout);
    assert_eq!(value["slice"], "my-slice");
    assert_eq!(value["requirements"][0]["id"], "REQ-001");
    assert_eq!(value["requirements"][0]["title"], "Password reset request");

    // Text mode prints the concise human view.
    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["slice", "model", "show", "my-slice"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(stdout.contains("slice: my-slice"), "header must name the slice, got: {stdout}");
    assert!(
        stdout.contains("REQ-001 [agreed] Password reset request"),
        "requirement line must render id/status/title, got: {stdout}"
    );
    assert!(
        stdout.contains("sources: legacy-monolith"),
        "requirement line must render sources, got: {stdout}"
    );
}

#[test]
fn model_show_fails_without_model() {
    let project = stage_slice_with_spec(CLEAN_SPEC_MD, Some(PLAN_WITH_LEGACY_MONOLITH));
    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "slice", "model", "show", "my-slice"])
        .assert()
        .failure();
    assert_eq!(assert.get_output().status.code(), Some(2));
    let value = parse_json(&assert.get_output().stderr);
    assert_eq!(value["error"], "slice-model-missing");
}
