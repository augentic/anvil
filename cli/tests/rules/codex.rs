//! Integration tests for shared codex distribution (RM-07).
//!
//! Cover `specify init` populating the project codex cache, `specify
//! rules sync` refreshing it, and `specify rules export` resolving the
//! distributed shared rules without a `--rules-root` flag.

use std::fs;
use std::path::{Path, PathBuf};

use tempfile::tempdir;

use crate::common::{copy_dir, expected_cache_dir, omnia_schema_dir, parse_json, specify_cmd};

/// Write a schema-valid shared rule under
/// `<root>/adapters/shared/rules/universal/<id>.md`.
fn write_universal_rule(root: &Path, id: &str) {
    let path = root.join(format!("adapters/shared/rules/universal/{id}.md"));
    fs::create_dir_all(path.parent().expect("rule parent")).expect("mkdir rule dir");
    fs::write(
        &path,
        format!(
            "---\nid: {id}\ntitle: {id} fixture\nseverity: important\ntrigger: Synthetic codex distribution integration fixture trigger sentence.\n---\n\n## Rule\n\nBody for {id}.\n"
        ),
    )
    .expect("write rule fixture");
}

/// Assemble a synthetic framework source repo (omnia target adapter
/// plus a shared `universal/` pack) and return the adapter dir path.
fn synthetic_source(root: &Path) -> PathBuf {
    let omnia = root.join("adapters/targets/omnia");
    copy_dir(&omnia_schema_dir(), &omnia);
    write_universal_rule(root, "UNI-901");
    omnia
}

#[test]
fn codex_cache_export_no_rules_root() {
    let src = tempdir().unwrap();
    let omnia = synthetic_source(src.path());
    let project = tempdir().unwrap();

    specify_cmd()
        .current_dir(project.path())
        .args(["init"])
        .arg(&omnia)
        .args(["--name", "demo"])
        .assert()
        .success();

    let cached =
        expected_cache_dir(project.path()).join("codex/adapters/shared/rules/universal/UNI-901.md");
    assert!(cached.is_file(), "init must distribute the shared codex into the cache");

    // `rules export` resolves the distributed shared rule with NO
    // `--rules-root`: the resolver picks up the codex cache rung.
    let assert = specify_cmd()
        .current_dir(project.path())
        .args(["--format", "json", "rules", "export", "--target", "omnia"])
        .assert()
        .success();
    let value = parse_json(&assert.get_output().stdout);
    let rules = value["rules"].as_array().expect("rules array in export envelope");
    assert!(
        rules.iter().any(|r| r["rule-id"] == "UNI-901"),
        "exported codex must carry the distributed UNI-901 rule, got:\n{value:#}"
    );
}

#[test]
fn rules_sync_refreshes_codex_cache() {
    let src = tempdir().unwrap();
    let omnia = synthetic_source(src.path());
    let project = tempdir().unwrap();

    specify_cmd()
        .current_dir(project.path())
        .args(["init"])
        .arg(&omnia)
        .args(["--name", "demo"])
        .assert()
        .success();

    // Drop the distributed cache, then refresh it via `rules sync`
    // (which re-resolves the recorded adapter source).
    fs::remove_dir_all(expected_cache_dir(project.path()).join("codex")).expect("rm codex cache");

    let assert = specify_cmd()
        .current_dir(project.path())
        .args(["--format", "json", "rules", "sync"])
        .assert()
        .success();
    let value = parse_json(&assert.get_output().stdout);
    assert_eq!(value["distributed"], true, "sync must redistribute, got:\n{value:#}");

    let cached =
        expected_cache_dir(project.path()).join("codex/adapters/shared/rules/universal/UNI-901.md");
    assert!(cached.is_file(), "rules sync must repopulate the codex cache");
}

#[test]
fn rules_sync_on_hub_without_source_errors() {
    let tmp = tempdir().unwrap();
    specify_cmd()
        .current_dir(tmp.path())
        .args(["init", "--name", "platform-workspace", "--workspace"])
        .assert()
        .success();

    let assert = specify_cmd().current_dir(tmp.path()).args(["rules", "sync"]).assert().failure();
    let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("utf8");
    assert!(
        stderr.contains("declares no adapter") || stderr.contains("rules-sync-no-adapter"),
        "workspace `rules sync` must explain the missing adapter, got stderr:\n{stderr}"
    );
}
