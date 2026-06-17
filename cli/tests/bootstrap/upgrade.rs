//! Integration test for `specify upgrade --dry-run --format json`.
//!
//! Drives the command end-to-end against a forced `--channel cargo` and
//! an injected release tag (`SPECIFY_RELEASE_TAG`) so the envelope is
//! deterministic and the test never touches `gh` or the network. The
//! per-channel planning, classification, and JSON-probe parsing are
//! unit-tested in the workflow crate; this asserts the wire shape Change
//! G's `/spec:init` skill parses (`channel`, `to`, `commands`).

use crate::common::{parse_json, specify_cmd};

#[test]
fn dry_run_reports_channel_commands() {
    let tmp = tempfile::tempdir().expect("tempdir");

    let assert = specify_cmd()
        .current_dir(tmp.path())
        .env("SPECIFY_RELEASE_TAG", "v9.9.9")
        .args(["--format", "json", "upgrade", "--channel", "cargo", "--dry-run"])
        .assert()
        .success();
    let body = parse_json(&assert.get_output().stdout);

    assert_eq!(body["version"], 1);
    assert_eq!(body["channel"], "cargo");
    assert_eq!(body["to"], "9.9.9");
    assert_eq!(body["dry-run"], true);
    assert_eq!(body["applied"], false);
    assert_eq!(body["head-fallback"], false);
    assert_eq!(body["journaled"], false);

    let commands = body["commands"].as_array().expect("commands array");
    assert_eq!(commands.len(), 1, "cargo channel plans one command");
    assert_eq!(commands[0]["program"], "cargo");
    let args: Vec<&str> =
        commands[0]["args"].as_array().expect("args").iter().map(|a| a.as_str().unwrap()).collect();
    assert_eq!(
        args,
        ["install", "--git", "https://github.com/augentic/specify-cli", "--tag", "v9.9.9"]
    );

    assert!(
        !tmp.path().join(".specify/journal.jsonl").exists(),
        "dry-run must not append a journal event"
    );
}

#[test]
fn upgrade_without_consent_refuses() {
    let tmp = tempfile::tempdir().expect("tempdir");

    let assert = specify_cmd()
        .current_dir(tmp.path())
        .env("SPECIFY_RELEASE_TAG", "v9.9.9")
        .args(["--format", "json", "upgrade", "--channel", "cargo"])
        .assert()
        .failure();
    let body = parse_json(&assert.get_output().stderr);
    assert_eq!(body["error"], "upgrade-consent-required");
}
