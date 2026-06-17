//! Integration tests for `specify target resolve`.
//!
//! Mirrors the target-adapter loader exposed by
//! `crates/workflow/src/adapter/`. The CLI verb is a thin
//! `TargetAdapter::resolve(adapter_ref, project_dir)` wrapper.

use std::fs;
use std::path::PathBuf;

mod common;
use common::{Project, copy_dir, expected_cache_dir, parse_stdout, repo_root, specify_cmd};

fn plugin_fixtures_root() -> PathBuf {
    repo_root().join("crates/workflow/tests/fixtures/plugins")
}

fn stage_target_fixture(project: &Project, name: &str) {
    let src = plugin_fixtures_root().join("adapters").join("targets").join(name);
    let dst = project.root().join("adapters").join("targets").join(name);
    copy_dir(&src, &dst);
}

#[test]
fn resolve_local_returns_manifest() {
    let project = Project::init();
    // `Project::init()` seeds the out-of-tree manifest cache with
    // `manifests/targets/omnia/`; remove it so the local probe wins.
    let cached = expected_cache_dir(project.root()).join("manifests/targets/omnia");
    if cached.exists() {
        fs::remove_dir_all(&cached).expect("clear cached omnia");
    }
    stage_target_fixture(&project, "omnia");

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "target", "resolve", "omnia"])
        .arg("--project-dir")
        .arg(project.root())
        .assert()
        .success();

    let actual = parse_stdout(&assert.get_output().stdout, project.root());
    assert_eq!(actual["axis"], "targets");
    assert_eq!(actual["name"], "omnia");
    assert_eq!(actual["location"], "local");
    let ops: Vec<&str> =
        actual["operations"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
    // After the `operations[]` collapse (review 1.A1), the envelope
    // derives operations from `briefs.keys()` — a BTreeMap, so order is
    // ascending kebab-name: build < merge < shape.
    assert_eq!(ops, vec!["build", "merge", "shape"]);
    let resolved = actual["resolved-path"].as_str().expect("resolved-path str");
    assert!(
        resolved.ends_with("adapters/targets/omnia"),
        "resolved-path {resolved} must end with targets/omnia"
    );
    let briefs_dir = actual["briefs-dir"].as_str().expect("briefs-dir str");
    assert_eq!(
        briefs_dir,
        format!("{resolved}/briefs"),
        "briefs-dir must be the resolved adapter root joined with briefs/"
    );
}

#[test]
fn resolve_accepts_version_suffix() {
    // workflow §CLI surface: `specify target resolve <value>` takes
    // either `<name>` or `<name>@<semver>` (RFC-47). The semver pin is
    // matched against the installed identity; the omnia fixture is
    // `1.0.0`, so the matching pin resolves and the envelope reports the
    // bare kebab name.
    let project = Project::init();
    stage_target_fixture(&project, "omnia");

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "target", "resolve", "omnia@1.0.0"])
        .arg("--project-dir")
        .arg(project.root())
        .assert()
        .success();

    let actual = parse_stdout(&assert.get_output().stdout, project.root());
    assert_eq!(actual["name"], "omnia");
}

#[test]
fn adapter_group_exposes_build_and_publish() {
    // `specify adapter` is un-retired at RFC-48 as the packaging group
    // (`build` + `publish`). `--help` must exit 0 and the contract dump
    // must declare both verbs.
    specify_cmd().args(["adapter", "--help"]).assert().success();
    let verbs = common::contract_dump_verbs(&["adapter"]);
    for verb in ["build", "publish"] {
        assert!(verbs.iter().any(|v| v == verb), "adapter must declare `{verb}`, got: {verbs:?}");
    }
}

#[test]
fn retired_adapter_resolve_rejected() {
    // The old `specify adapter resolve` is gone (resolution moved to
    // `source`/`target`); clap rejects the unknown subcommand with 2.
    let assert = specify_cmd().arg("adapter").arg("resolve").arg("omnia").assert().failure();
    let code = assert.get_output().status.code().expect("exit code");
    assert_eq!(
        code, 2,
        "clap must reject the removed `adapter resolve` verb with exit 2, got {code}"
    );
}

#[test]
fn retired_change_verb_rejected_by_clap() {
    // `specify change *` retires at 2.0 (workflow §What was cut and why).
    let assert = specify_cmd().arg("change").arg("draft").arg("demo").assert().failure();
    let code = assert.get_output().status.code().expect("exit code");
    assert_eq!(code, 2, "clap must reject the retired `change` verb with exit 2, got {code}");
}
