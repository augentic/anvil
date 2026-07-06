//! Integration tests for `specify adapter build`.
//!
//! The packing path is local and byte-deterministic, so these tests drive it
//! against throw-away adapter trees under `tempfile::TempDir`. The OCI
//! `publish` flow (network) is out of scope.

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
    assert!(body["layer_bytes"].as_u64().is_some_and(|n| n > 0), "non-empty layer");
    assert!(body["digest"].as_str().is_some_and(|d| !d.is_empty()), "digest reported");
}

#[test]
fn build_excludes_rust_source_trees() {
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
    let baseline = parse_json(&assert.get_output().stdout);

    // Rust source trees are excluded from the packed layer, so adding one
    // must not perturb the byte-deterministic digest.
    write(&dir.join("extension/src/lib.rs"), "fn main() {}\n");
    let assert = specify_cmd()
        .current_dir(tmp.path())
        .args(["--format", "json", "adapter", "build", "--path"])
        .arg(&dir)
        .assert()
        .success();
    let with_source = parse_json(&assert.get_output().stdout);
    assert_eq!(baseline["digest"], with_source["digest"], "source trees excluded from layer");
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
