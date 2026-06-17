//! `slice drop` CLI tests.

use crate::support::*;

#[test]
fn drop_transitions_and_archives() {
    let project = Project::init();
    specify_cmd()
        .current_dir(project.root())
        .args(["slice", "create", "my-slice"])
        .assert()
        .success();

    let assert = specify_cmd()
        .current_dir(project.root())
        .args([
            "--format",
            "json",
            "slice",
            "drop",
            "my-slice",
            "--reason",
            "Needs design call-out",
        ])
        .assert()
        .success();
    let value = parse_json(&assert.get_output().stdout);
    assert_eq!(value["status"], "dropped");
    assert_eq!(value["drop-reason"], "Needs design call-out");
    let archive_path = value["archive-path"].as_str().unwrap();
    assert!(archive_path.ends_with("-my-slice"));

    // `metadata.yaml` inside the archive should reflect the drop.
    let archived_meta = fs::read_to_string(format!("{archive_path}/metadata.yaml")).unwrap();
    assert!(archived_meta.contains("status: dropped"));
    assert!(archived_meta.contains("drop-reason: Needs design call-out"));
    assert!(archived_meta.contains("dropped-at:"));
}
