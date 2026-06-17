//! Integration test for the dedicated `scenario` discovery pass.
//!
//! Proves the scoped fact family: a staged `evals/scenarios/*.md`
//! file is discovered into `model.scenarios` (with `id` / `fields`
//! projected), and is kept OUT of `model.files` so no other rule's
//! candidate set changes.

use std::fs;
use std::path::Path;

use specify_standards::lint::ScanProfile;
use specify_standards::lint::index::build;

fn write_scenario(project: &Path, name: &str, id: &str) {
    let content = format!(
        "---\nid: {id}\nowner: spec\nkind: skill\nentrypoint: /spec:refine\nstages: [refine, build]\nisolation: fresh-project\nexpected-artifacts: [spec.md]\n---\n\nScenario ID: `{id}`\n"
    );
    let path = project.join(format!("evals/scenarios/{name}"));
    fs::create_dir_all(path.parent().expect("parent")).expect("scenario dir");
    fs::write(&path, content).expect("write scenario");
}

#[test]
fn discovers_scenario_into_dedicated_family() {
    let tmp = tempfile::tempdir().expect("tmp");
    write_scenario(tmp.path(), "refine.md", "refine-happy-path");

    let model = build(tmp.path(), ScanProfile::Framework, &[], &[]).expect("framework build");

    let scenario = model
        .scenarios
        .iter()
        .find(|s| s.path == "evals/scenarios/refine.md")
        .expect("staged scenario appears in model.scenarios");
    assert_eq!(scenario.id.as_deref(), Some("refine-happy-path"));
    assert_eq!(scenario.stages, vec!["refine".to_string(), "build".to_string()]);
    assert_eq!(scenario.expected_artifacts, vec!["spec.md".to_string()]);
    assert_eq!(scenario.body_id.as_deref(), Some("refine-happy-path"));
    assert_eq!(scenario.fields.get("owner").and_then(|v| v.as_str()), Some("spec"));
}

#[test]
fn scenario_file_is_kept_out_of_files() {
    let tmp = tempfile::tempdir().expect("tmp");
    write_scenario(tmp.path(), "refine.md", "refine-happy-path");

    let model = build(tmp.path(), ScanProfile::Framework, &[], &[]).expect("framework build");

    assert!(
        !model.files.iter().any(|f| f.path == "evals/scenarios/refine.md"),
        "eval scenario files must not enter model.files (zero blast radius)"
    );
}
