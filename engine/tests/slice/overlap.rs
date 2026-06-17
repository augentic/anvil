//! `slice overlap` CLI tests.

use crate::support::*;

#[test]
fn overlap_reports_shared_adapters() {
    let project = Project::init();
    // Two active slices both claim `login`.
    specify_cmd().current_dir(project.root()).args(["slice", "create", "first"]).assert().success();
    specify_cmd()
        .current_dir(project.root())
        .args(["slice", "create", "second"])
        .assert()
        .success();
    specify_cmd()
        .current_dir(project.root())
        .args(["slice", "touched-specs", "first", "--set", "login:new,oauth:new"])
        .assert()
        .success();
    specify_cmd()
        .current_dir(project.root())
        .args(["slice", "touched-specs", "second", "--set", "login:modified"])
        .assert()
        .success();

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "slice", "overlap", "first"])
        .assert()
        .success();
    let value = parse_json(&assert.get_output().stdout);
    let overlaps = value["overlaps"].as_array().unwrap();
    assert_eq!(overlaps.len(), 1);
    assert_eq!(overlaps[0]["capability"], "login");
    assert_eq!(overlaps[0]["other-slice"], "second");
    assert_eq!(overlaps[0]["our-spec-type"], "new");
    assert_eq!(overlaps[0]["other-spec-type"], "modified");
}

#[test]
fn overlap_empty_for_disjoint_slices() {
    let project = Project::init();
    specify_cmd().current_dir(project.root()).args(["slice", "create", "alpha"]).assert().success();
    specify_cmd().current_dir(project.root()).args(["slice", "create", "beta"]).assert().success();
    specify_cmd()
        .current_dir(project.root())
        .args(["slice", "touched-specs", "alpha", "--set", "aa:new"])
        .assert()
        .success();
    specify_cmd()
        .current_dir(project.root())
        .args(["slice", "touched-specs", "beta", "--set", "bb:new"])
        .assert()
        .success();

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "slice", "overlap", "alpha"])
        .assert()
        .success();
    let value = parse_json(&assert.get_output().stdout);
    assert!(value["overlaps"].as_array().unwrap().is_empty());
}
