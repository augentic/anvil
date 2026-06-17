//! Integration tests for `specify source preview` (`specify source preview` contract).

use std::fs;
use std::path::PathBuf;

use tempfile::tempdir;

use crate::common::{copy_dir, parse_stderr, parse_stdout, repo_root, specify_cmd};

fn plugin_fixtures_root() -> PathBuf {
    repo_root().join("crates/workflow/tests/fixtures/plugins")
}

fn stage_source_adapter(root: &std::path::Path, name: &str) {
    let src = plugin_fixtures_root().join("adapters").join("sources").join(name);
    let dst = root.join("adapters").join("sources").join(name);
    copy_dir(&src, &dst);
}

#[test]
fn preview_succeeds_without_specify_dir() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path();
    stage_source_adapter(root, "typescript");

    let source_dir = root.join("my-source");
    fs::create_dir_all(&source_dir).expect("create source dir");

    let assert = specify_cmd()
        .current_dir(root)
        .args(["--format", "json", "source", "preview", "typescript"])
        .arg("--source")
        .arg(&source_dir)
        .arg("--project-dir")
        .arg(root)
        .assert()
        .success();

    let actual = parse_stdout(&assert.get_output().stdout, root);
    assert_eq!(actual["adapter"], "typescript");
    assert_eq!(actual["version"], "1.0.0");

    let briefs = actual["briefs"].as_array().expect("briefs array");
    assert_eq!(briefs.len(), 2);
    let ops: Vec<&str> = briefs.iter().map(|b| b["operation"].as_str().unwrap()).collect();
    assert!(ops.contains(&"survey"));
    assert!(ops.contains(&"extract"));
}

#[test]
fn preview_creates_output_directory() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path();
    stage_source_adapter(root, "typescript");

    let source_dir = root.join("my-source");
    fs::create_dir_all(&source_dir).expect("create source dir");

    let out_dir = root.join("custom-out");

    specify_cmd()
        .current_dir(root)
        .args(["--format", "json", "source", "preview", "typescript"])
        .arg("--source")
        .arg(&source_dir)
        .arg("--out")
        .arg(&out_dir)
        .arg("--project-dir")
        .arg(root)
        .assert()
        .success();

    assert!(out_dir.join("evidence").is_dir(), "evidence/ subdirectory must be created");
    assert!(!root.join(".specify").exists(), "no .specify/ residue");
}

#[test]
fn default_out_creates_preview() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path();
    stage_source_adapter(root, "typescript");

    let source_dir = root.join("my-source");
    fs::create_dir_all(&source_dir).expect("create source dir");

    specify_cmd()
        .current_dir(root)
        .args(["source", "preview", "typescript"])
        .arg("--source")
        .arg(&source_dir)
        .arg("--project-dir")
        .arg(root)
        .assert()
        .success();

    assert!(
        root.join(".specify-preview/evidence").is_dir(),
        "default .specify-preview/evidence/ must be created"
    );
}

#[test]
fn preview_passes_leads_through() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path();
    stage_source_adapter(root, "typescript");

    let source_dir = root.join("my-source");
    fs::create_dir_all(&source_dir).expect("create source dir");

    let assert = specify_cmd()
        .current_dir(root)
        .args(["--format", "json", "source", "preview", "typescript"])
        .arg("--source")
        .arg(&source_dir)
        .args(["--lead", "login-screen", "--lead", "settings"])
        .arg("--project-dir")
        .arg(root)
        .assert()
        .success();

    let actual = parse_stdout(&assert.get_output().stdout, root);
    let leads = actual["leads"].as_array().expect("leads array");
    assert_eq!(leads.len(), 2);
    assert_eq!(leads[0], "login-screen");
    assert_eq!(leads[1], "settings");
}

#[test]
fn preview_fails_when_source_path_missing() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path();
    stage_source_adapter(root, "typescript");

    let assert = specify_cmd()
        .current_dir(root)
        .args(["--format", "json", "source", "preview", "typescript"])
        .arg("--source")
        .arg(root.join("nonexistent"))
        .arg("--project-dir")
        .arg(root)
        .assert()
        .failure();

    let stderr = parse_stderr(&assert.get_output().stderr, root);
    assert_eq!(stderr["error"], "argument");
    assert_eq!(stderr["exit-code"], 2);
}

#[test]
fn preview_fails_when_adapter_not_found() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path();

    let source_dir = root.join("my-source");
    fs::create_dir_all(&source_dir).expect("create source dir");

    let assert = specify_cmd()
        .current_dir(root)
        .args(["--format", "json", "source", "preview", "no-such-adapter"])
        .arg("--source")
        .arg(&source_dir)
        .arg("--project-dir")
        .arg(root)
        .assert()
        .failure();

    let stderr = parse_stderr(&assert.get_output().stderr, root);
    assert_eq!(stderr["error"], "adapter-not-found");
    assert_eq!(stderr["exit-code"], 1);
}
