//! `slice validate` spec file-location gate.

use crate::support::*;

#[test]
fn root_spec_without_canonical() {
    let project = Project::init().with_schemas();
    specify_cmd()
        .current_dir(project.root())
        .args(["slice", "create", "my-slice"])
        .assert()
        .success();
    let slice_dir = project.slices_dir().join("my-slice");
    fs::write(slice_dir.join("spec.md"), CLEAN_SPEC_MD).expect("write root spec.md");
    fs::remove_dir_all(slice_dir.join("specs")).expect("remove specs dir created by slice create");

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "slice", "validate", "my-slice"])
        .assert()
        .failure();
    assert_eq!(assert.get_output().status.code(), Some(2));
    let value = parse_json(&assert.get_output().stderr);
    assert_eq!(value["error"], "slice-pre-adapter-gate");
    let detail = find_finding_impact(assert.get_output(), "specs.file-location");
    assert!(
        detail.contains("specs/<domain>/spec.md"),
        "detail must name the canonical layout, got: {detail}"
    );
    assert!(detail.contains("slice root"), "detail must mention the slice root, got: {detail}");
}

#[test]
fn skipped_when_canonical_exists() {
    let project = stage_slice_with_spec(CLEAN_SPEC_MD, Some(PLAN_WITH_LEGACY_MONOLITH));
    let slice_dir = project.slices_dir().join("my-slice");
    fs::write(slice_dir.join("spec.md"), "stale root copy").expect("write root spec.md");

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "slice", "validate", "my-slice"])
        .assert();
    let stderr = assert.get_output().stderr.clone();
    if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&stderr)
        && let Some(results) = value["results"].as_array()
    {
        for r in results {
            let rule_id = r["rule-id"].as_str().unwrap_or("");
            assert_ne!(
                rule_id, "specs.file-location",
                "file-location gate must not fire when canonical specs exist"
            );
        }
    }
}

#[test]
fn skipped_when_no_root_spec() {
    let project = Project::init().with_schemas();
    specify_cmd()
        .current_dir(project.root())
        .args(["slice", "create", "my-slice"])
        .assert()
        .success();
    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "slice", "validate", "my-slice"])
        .assert();
    let stderr = assert.get_output().stderr.clone();
    if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&stderr)
        && let Some(results) = value["results"].as_array()
    {
        for r in results {
            let rule_id = r["rule-id"].as_str().unwrap_or("");
            assert_ne!(
                rule_id, "specs.file-location",
                "file-location gate must not fire when no root spec.md exists"
            );
        }
    }
}
