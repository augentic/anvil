//! Integration tests for `specify adapter build`.
//!
//! The packing path is local and byte-deterministic, so these tests drive it
//! against throw-away adapter trees under `tempfile::TempDir`. The OCI
//! `publish` flow (network) is out of scope. Extension compile coverage
//! includes a workspace-member fixture under `tests/fixtures/adapter-build-workspace/`.

use std::fs;
use std::path::Path;

use common::{parse_json, parse_stderr, specify_cmd};
use tempfile::tempdir;

use crate::common;

fn write(path: &Path, body: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    fs::write(path, body).expect("write fixture");
}

#[test]
fn build_prose_only_adapter() {
    let tmp = tempdir().expect("tempdir");
    let dir = tmp.path().join("omnia");
    write(&dir.join("adapter.yaml"), "name: omnia\nversion: 1.2.0\n");
    write(&dir.join("briefs/build.md"), "# build\n");

    let assert = specify_cmd()
        .current_dir(tmp.path())
        .args(["--format", "json", "adapter", "build", "--path"])
        .arg(&dir)
        .assert()
        .success();

    let body = parse_json(&assert.get_output().stdout);
    assert_eq!(body["name"], "omnia");
    assert_eq!(body["version"], "1.2.0");
    assert_eq!(body["extension_declared"], false);
    assert_eq!(body["wasm_built"], false);
    assert_eq!(body["dry_run"], false);
    assert!(body["layer_bytes"].as_u64().is_some_and(|n| n > 0), "non-empty layer");
    assert!(body["digest"].as_str().is_some_and(|d| !d.is_empty()), "digest reported");
}

#[test]
fn build_dry_run_skips_wasm() {
    let tmp = tempdir().expect("tempdir");
    let dir = tmp.path().join("omnia");
    write(&dir.join("adapter.yaml"), "name: omnia\nversion: 1.2.0\n");

    let assert = specify_cmd()
        .current_dir(tmp.path())
        .args(["--format", "json", "adapter", "build", "--dry-run", "--path"])
        .arg(&dir)
        .assert()
        .success();

    let body = parse_json(&assert.get_output().stdout);
    assert_eq!(body["dry_run"], true);
    assert_eq!(body["wasm_built"], false);
}

#[test]
fn build_committed_wasm_reused() {
    let tmp = tempdir().expect("tempdir");
    let dir = tmp.path().join("omnia");
    // Declares an extension but ships a committed adapter.wasm, so build
    // reports it without invoking cargo.
    write(&dir.join("adapter.yaml"), "name: omnia\nversion: 1.2.0\nextension:\n  name: omnia\n");
    write(&dir.join("adapter.wasm"), "fake component bytes");

    let assert = specify_cmd()
        .current_dir(tmp.path())
        .args(["--format", "json", "adapter", "build", "--path"])
        .arg(&dir)
        .assert()
        .success();

    let body = parse_json(&assert.get_output().stdout);
    assert_eq!(body["extension_declared"], true);
    assert_eq!(body["wasm_built"], false, "committed wasm is reused, not recompiled");
}

#[test]
fn build_missing_manifest_fails() {
    let tmp = tempdir().expect("tempdir");
    let dir = tmp.path().join("empty");
    fs::create_dir_all(&dir).expect("mkdir");

    let assert = specify_cmd()
        .current_dir(tmp.path())
        .args(["--format", "json", "adapter", "build", "--path"])
        .arg(&dir)
        .assert()
        .failure();

    assert_eq!(
        parse_stderr(&assert.get_output().stderr, tmp.path())["error"],
        "adapter-build-failed",
    );
}

#[test]
fn build_extension_without_crate_fails() {
    let tmp = tempdir().expect("tempdir");
    let dir = tmp.path().join("omnia");
    // Declares an extension, ships no committed wasm, and has no extension/
    // crate to compile — build refuses rather than shelling a doomed cargo.
    write(&dir.join("adapter.yaml"), "name: omnia\nversion: 1.2.0\nextension:\n  name: omnia\n");

    let assert = specify_cmd()
        .current_dir(tmp.path())
        .args(["--format", "json", "adapter", "build", "--path"])
        .arg(&dir)
        .assert()
        .failure();

    assert_eq!(
        parse_stderr(&assert.get_output().stderr, tmp.path())["error"],
        "adapter-build-failed",
    );
}

#[test]
fn build_refresh_in_workspace() {
    let fixture = common::repo_root().join("tests/fixtures/adapter-build-workspace");
    let adapter = fixture.join("adapters/demo");
    let wasm = adapter.join("adapter.wasm");

    if wasm.exists() {
        fs::remove_file(&wasm).expect("remove stale adapter.wasm");
    }

    let assert = specify_cmd()
        .current_dir(&fixture)
        .args([
            "--format",
            "json",
            "adapter",
            "build",
            "--path",
            "adapters/demo",
            "--refresh-extension",
        ])
        .assert()
        .success();

    let body = parse_json(&assert.get_output().stdout);
    assert_eq!(body["name"], "demo");
    assert_eq!(body["extension_declared"], true);
    assert_eq!(body["wasm_built"], true);
    assert!(
        wasm.is_file() && fs::metadata(&wasm).is_ok_and(|meta| meta.len() > 0),
        "adapter.wasm must be written beside adapter.yaml"
    );

    if wasm.exists() {
        fs::remove_file(&wasm).expect("remove generated adapter.wasm");
    }
}
