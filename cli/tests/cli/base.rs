//! Cross-cutting binary-contract tests for the `specify` CLI.
//!
//! These are the subcommand-agnostic invariants every release must
//! preserve: the top-level `--help` shape, exit-code contracts that
//! aren't tied to a single subcommand, and JSON-envelope skeletons
//! that any verb may surface. Per-subcommand integration coverage
//! lives in dedicated `tests/<subcommand>.rs` files.

use std::fs;

use tempfile::tempdir;

use crate::common::{contract_dump_verbs, omnia_schema_dir, specify_cmd};

#[test]
fn help_exits_zero_and_prints_usage() {
    // No exact clap wording: assert exit 0 and that the help text lists
    // every top-level verb the contract dump declares.
    let assert = specify_cmd().arg("--help").assert().success();
    let output = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8 stdout");
    for verb in contract_dump_verbs(&[]) {
        assert!(output.contains(&verb), "--help must list `{verb}`, got:\n{output}");
    }
}

#[test]
fn version_too_old_exits_three_json() {
    // Generic exit-code 3 + JSON error-envelope contract: pin the
    // `specify-version-too-old` shape via a real project. Routed
    // through `slice validate` because that path runs the version
    // gate after a successful init; the gate itself is subcommand-
    // agnostic and the assertions below only touch the envelope.
    let tmp = tempdir().unwrap();
    // Fresh init to produce a real project.
    specify_cmd()
        .current_dir(tmp.path())
        .args(["init"])
        .arg(omnia_schema_dir())
        .args(["--name", "demo"])
        .assert()
        .success();

    // Pin a version far in the future.
    let config_path = tmp.path().join(".specify/project.yaml");
    let original = fs::read_to_string(&config_path).unwrap();
    let edited = original.replace(
        &format!("specify_version: {}", env!("CARGO_PKG_VERSION")),
        "specify_version: 99.0.0",
    );
    fs::write(&config_path, edited).unwrap();

    let assert = specify_cmd()
        .current_dir(tmp.path())
        .args(["--format", "json", "slice", "validate", "."])
        .assert()
        .failure();
    let code = assert.get_output().status.code().expect("process exited with a code");
    assert_eq!(code, 3, "expected exit code 3 (version too old)");

    // Failure envelopes are written to stderr.
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("utf8");
    let value: serde_json::Value = serde_json::from_str(&stderr).expect("stderr is JSON");
    assert_eq!(value["error"], "specify-version-too-old");
    assert_eq!(value["exit-code"], 3);
}

#[test]
fn older_pin_loads_fine() {
    // A pin older than the binary is legal: pre-1.0 majors are re-init,
    // not migration, so config load succeeds and the command proceeds to
    // ordinary slice validation (exit 2 on the bogus slice name).
    let tmp = tempdir().unwrap();
    specify_cmd()
        .current_dir(tmp.path())
        .args(["init"])
        .arg(omnia_schema_dir())
        .args(["--name", "demo"])
        .assert()
        .success();

    let config_path = tmp.path().join(".specify/project.yaml");
    let original = fs::read_to_string(&config_path).unwrap();
    let edited = original.replace(
        &format!("specify_version: {}", env!("CARGO_PKG_VERSION")),
        "specify_version: 0.0.1",
    );
    assert_ne!(original, edited, "fixture must repin specify_version");
    fs::write(&config_path, edited).unwrap();

    let assert = specify_cmd()
        .current_dir(tmp.path())
        .args(["--format", "json", "slice", "validate", "."])
        .assert()
        .failure();
    let code = assert.get_output().status.code().expect("process exited with a code");
    assert_eq!(code, 2, "older pin loads; the failure is ordinary validation");
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("utf8");
    let value: serde_json::Value = serde_json::from_str(&stderr).expect("stderr is JSON");
    assert_eq!(value["error"], "slice-validation-failed");
}
