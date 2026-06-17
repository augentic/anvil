//! Integration tests for `specify source resolve`.
//!
//! Mirrors the source-adapter loader exposed by
//! `crates/workflow/src/plugin/`. The CLI verb is a thin
//! `Plugin::resolve(Axis::Source, …)` wrapper; the cases below pin
//! the wire shape skill bodies and downstream callers rely on.

use std::path::PathBuf;

use crate::common::{Project, copy_dir, parse_stderr, parse_stdout, repo_root, specify_cmd};

fn plugin_fixtures_root() -> PathBuf {
    repo_root().join("crates/workflow/tests/fixtures/plugins")
}

fn stage_source_fixture(project: &Project, name: &str) {
    let src = plugin_fixtures_root().join("adapters").join("sources").join(name);
    let dst = project.root().join("adapters").join("sources").join(name);
    copy_dir(&src, &dst);
}

#[test]
fn resolve_local_returns_manifest() {
    let project = Project::init();
    stage_source_fixture(&project, "typescript");

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "source", "resolve", "typescript"])
        .arg("--project-dir")
        .arg(project.root())
        .assert()
        .success();

    let actual = parse_stdout(&assert.get_output().stdout, project.root());
    assert_eq!(actual["axis"], "sources");
    assert_eq!(actual["name"], "typescript");
    assert_eq!(actual["location"], "local");
    let operations = actual["operations"].as_array().expect("operations array");
    let ops: Vec<&str> = operations.iter().map(|v| v.as_str().unwrap()).collect();
    assert_eq!(ops, vec!["extract", "survey"]);
    let resolved = actual["resolved-path"].as_str().expect("resolved-path str");
    assert!(
        resolved.ends_with("adapters/sources/typescript"),
        "resolved-path {resolved} must end with sources/typescript"
    );
    let briefs_dir = actual["briefs-dir"].as_str().expect("briefs-dir str");
    assert_eq!(
        briefs_dir,
        format!("{resolved}/briefs"),
        "briefs-dir must be the resolved adapter root joined with briefs/"
    );
}

#[test]
fn resolve_missing_emits_not_found() {
    let project = Project::init();

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "source", "resolve", "no-such-source"])
        .arg("--project-dir")
        .arg(project.root())
        .assert()
        .failure();
    let stderr = parse_stderr(&assert.get_output().stderr, project.root());
    assert_eq!(stderr["error"], "adapter-not-found");
    assert_eq!(stderr["exit-code"], 1);
}
