//! Integration tests for `specify target resolve`.
//!
//! Mirrors the target-adapter loader exposed by
//! `crates/workflow/src/adapter/`. The CLI verb is a thin
//! `TargetAdapter::resolve(adapter_ref, project_dir)` wrapper.

use common::{Project, expected_cache_dir, parse_stdout, specify_cmd, stage_store_component};

use crate::common;

#[test]
fn resolve_bare_returns_component() {
    // `Project::init()` mirrors the local `omnia.wasm` component into
    // the project component cache; the bare name resolves it there.
    let project = Project::init();

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
    assert_eq!(actual["location"], "dev");
    let ops: Vec<&str> =
        actual["operations"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
    // The envelope derives operations from the closed WIT contract, in
    // ascending kebab-name order: build < guidance < merge.
    assert_eq!(ops, vec!["build", "guidance", "merge"]);
    let resolved = actual["resolved-path"].as_str().expect("resolved-path str");
    assert!(
        resolved.ends_with("components/omnia.wasm"),
        "resolved-path {resolved} must end with the cached component file"
    );
    let expected = expected_cache_dir(project.root()).join("components/omnia.wasm");
    assert!(expected.is_file(), "init must have mirrored the component into the cache");
}

#[test]
fn resolve_accepts_version_suffix() {
    // workflow §CLI surface: `specify target resolve <value>` takes
    // either `<name>` or `<name>@<semver>` (RFC-47). The semver pin
    // resolves the single-file global store entry
    // (`<store>/<name>@<version>.wasm`), not the development probes.
    let project = Project::init();
    stage_store_component("omnia", "1.0.0");

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "target", "resolve", "omnia@1.0.0"])
        .arg("--project-dir")
        .arg(project.root())
        .assert()
        .success();

    let actual = parse_stdout(&assert.get_output().stdout, project.root());
    assert_eq!(actual["name"], "omnia");
    assert_eq!(actual["version"], "1.0.0");
    assert_eq!(actual["location"], "store");
    let resolved = actual["resolved-path"].as_str().expect("resolved-path str");
    assert!(
        resolved.ends_with("omnia@1.0.0.wasm"),
        "resolved-path {resolved} must be the single-file store entry"
    );
}

#[test]
fn retired_adapter_group_rejected() {
    // The `specify adapter {build,publish}` packaging group retired
    // with RFC-64 (publishing is `wkg publish` in the adapters repo);
    // clap rejects the unknown command with exit 2.
    let assert = specify_cmd().arg("adapter").arg("build").assert().failure();
    let code = assert.get_output().status.code().expect("exit code");
    assert_eq!(code, 2, "clap must reject the retired `adapter` group with exit 2, got {code}");
}

#[test]
fn retired_change_verb_rejected_by_clap() {
    // `specify change *` retires at 2.0 (workflow §What was cut and why).
    let assert = specify_cmd().arg("change").arg("draft").arg("demo").assert().failure();
    let code = assert.get_output().status.code().expect("exit code");
    assert_eq!(code, 2, "clap must reject the retired `change` verb with exit 2, got {code}");
}
