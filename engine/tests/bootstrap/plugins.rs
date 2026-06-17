//! Integration tests for `specify plugins {doctor, refresh}`.
//!
//! The status kernel, cache scan, and marketplace cross-reference are
//! unit-tested in the workflow crate; these drive the CLI surface
//! end-to-end. `CURSOR_HOME` is always pointed at a tempdir so the real
//! `~/.cursor` cache is never read or mutated. The `doctor` test stands
//! up a real git checkout so the expected-sha derivation (the
//! marketplace repo's `HEAD`) exercises the full `ok` / `drifted` path
//! alongside `missing` / `extra`.

use std::fs;
use std::path::Path;

use serde_json::Value;
use tempfile::TempDir;

use crate::common::{parse_json, run_git, specify_cmd};

/// Write a schema-valid marketplace.json declaring `plugins` at
/// `<project>/.cursor-plugin/marketplace.json`.
fn write_marketplace(project: &Path, name: &str, plugins: &[&str]) {
    let dir = project.join(".cursor-plugin");
    fs::create_dir_all(&dir).expect("mkdir .cursor-plugin");
    let entries: Vec<String> = plugins
        .iter()
        .map(|p| {
            format!(
                "{{ \"name\": \"{p}\", \"source\": \"{p}\", \"description\": \"The {p} plugin.\" }}"
            )
        })
        .collect();
    let json = format!(
        r#"{{
  "name": "{name}",
  "owner": {{ "name": "augentic", "email": "info@augentic.io" }},
  "metadata": {{ "description": "d", "version": "0.27.0", "pluginRoot": "plugins" }},
  "plugins": [ {} ]
}}"#,
        entries.join(", ")
    );
    fs::write(dir.join("marketplace.json"), json).expect("write marketplace.json");
}

/// Create `$CURSOR_HOME/plugins/cache/<name>/<plugin>/<sha>/`.
fn seed_cache_leaf(cursor_home: &Path, name: &str, plugin: &str, sha: &str) {
    let leaf = cursor_home.join("plugins/cache").join(name).join(plugin).join(sha);
    fs::create_dir_all(&leaf).expect("mkdir cache leaf");
}

/// Create a declared plugin dir with no `<sha>` leaf (a `missing` stub).
fn seed_cache_plugin_empty(cursor_home: &Path, name: &str, plugin: &str) {
    let dir = cursor_home.join("plugins/cache").join(name).join(plugin);
    fs::create_dir_all(&dir).expect("mkdir empty plugin dir");
}

#[test]
fn doctor_reports_ok_drifted_missing_extra() {
    let project = TempDir::new().expect("project tmp");
    let cursor = TempDir::new().expect("cursor tmp");

    // The marketplace repo is the project itself; HEAD is the expected
    // sha shared by every relative-path plugin.
    run_git(project.path(), &["init", "-q"]);
    fs::write(project.path().join("seed"), "x").expect("seed file");
    run_git(project.path(), &["add", "."]);
    run_git(project.path(), &["commit", "-qm", "init"]);
    let head = run_git(project.path(), &["rev-parse", "HEAD"]).trim().to_string();

    write_marketplace(project.path(), "augentic", &["spec", "capture", "client"]);
    // spec -> ok (cached == HEAD); capture -> drifted (stale sha);
    // client -> missing (no leaf); omnia -> extra (not declared).
    seed_cache_leaf(cursor.path(), "augentic", "spec", &head);
    seed_cache_leaf(cursor.path(), "augentic", "capture", "deadbeefdeadbeefdeadbeefdeadbeef");
    seed_cache_plugin_empty(cursor.path(), "augentic", "client");
    seed_cache_leaf(cursor.path(), "augentic", "omnia", "feedfacefeedfacefeedfacefeedface");

    let assert = specify_cmd()
        .current_dir(project.path())
        .env("CURSOR_HOME", cursor.path())
        .args(["--format", "json", "plugins", "doctor", "--project-dir"])
        .arg(project.path())
        .assert()
        .success();
    let body = parse_json(&assert.get_output().stdout);

    assert_eq!(body["version"], 1);
    let status_of = |name: &str| -> String {
        body["plugins"]
            .as_array()
            .expect("plugins array")
            .iter()
            .find(|p| p["name"] == name)
            .unwrap_or_else(|| panic!("plugin {name} present"))["status"]
            .as_str()
            .expect("status string")
            .to_string()
    };
    assert_eq!(status_of("spec"), "ok");
    assert_eq!(status_of("capture"), "drifted");
    assert_eq!(status_of("client"), "missing");
    assert_eq!(status_of("omnia"), "extra");

    let summary = &body["summary"];
    assert_eq!(summary["ok"], 1);
    assert_eq!(summary["drifted"], 1);
    assert_eq!(summary["missing"], 1);
    assert_eq!(summary["extra"], 1);
}

#[test]
fn doctor_degrades_to_present_without_git() {
    let project = TempDir::new().expect("project tmp");
    let cursor = TempDir::new().expect("cursor tmp");

    // No git checkout backing the marketplace -> expected unresolvable.
    write_marketplace(project.path(), "augentic", &["spec"]);
    seed_cache_leaf(cursor.path(), "augentic", "spec", "cafebabecafebabecafebabecafebabe");

    let assert = specify_cmd()
        .current_dir(project.path())
        .env("CURSOR_HOME", cursor.path())
        // Defang any ambient git repo above the tempdir.
        .env("GIT_CEILING_DIRECTORIES", project.path())
        .args(["--format", "json", "plugins", "doctor", "--project-dir"])
        .arg(project.path())
        .assert()
        .success();
    let body = parse_json(&assert.get_output().stdout);

    let spec = &body["plugins"].as_array().expect("plugins")[0];
    assert_eq!(spec["status"], "present");
    assert_eq!(spec["expected-sha"], Value::Null);
    assert_eq!(body["summary"]["present"], 1);
}

#[test]
fn refresh_deletes_scoped_cache() {
    let project = TempDir::new().expect("project tmp");
    let cursor = TempDir::new().expect("cursor tmp");

    // A discoverable `.specify/` root from the CWD so the event lands.
    fs::create_dir_all(project.path().join(".specify")).expect("mkdir .specify");
    fs::write(project.path().join(".specify/project.yaml"), "name: p\nadapter: omnia\n")
        .expect("seed project.yaml");

    write_marketplace(project.path(), "augentic", &["spec"]);
    seed_cache_leaf(cursor.path(), "augentic", "spec", "aaa");
    // A second marketplace's cache must survive.
    seed_cache_leaf(cursor.path(), "acme", "widget", "bbb");

    let assert = specify_cmd()
        .current_dir(project.path())
        .env("CURSOR_HOME", cursor.path())
        .args(["--format", "json", "plugins", "refresh", "--yes", "--project-dir"])
        .arg(project.path())
        .assert()
        .success();
    let body = parse_json(&assert.get_output().stdout);

    assert_eq!(body["version"], 1);
    assert_eq!(body["journaled"], true);
    assert_eq!(body["deleted-paths"].as_array().expect("deleted-paths").len(), 1);

    let augentic = cursor.path().join("plugins/cache/augentic");
    let acme = cursor.path().join("plugins/cache/acme");
    assert!(!augentic.exists(), "scoped cache removed");
    assert!(acme.exists(), "sibling marketplace cache survives");

    let journal =
        fs::read_to_string(project.path().join(".specify/journal.jsonl")).expect("journal written");
    let line = journal.lines().next().expect("one journal line");
    let event: Value = serde_json::from_str(line).expect("journal json");
    assert_eq!(event["event"], "plugins.refreshed");
    assert!(event["payload"]["marketplace"].as_str().is_some());
}

#[test]
fn refresh_without_consent_refuses() {
    let project = TempDir::new().expect("project tmp");
    let cursor = TempDir::new().expect("cursor tmp");
    write_marketplace(project.path(), "augentic", &["spec"]);
    seed_cache_leaf(cursor.path(), "augentic", "spec", "aaa");

    let assert = specify_cmd()
        .current_dir(project.path())
        .env("CURSOR_HOME", cursor.path())
        .args(["--format", "json", "plugins", "refresh", "--project-dir"])
        .arg(project.path())
        .assert()
        .failure();
    let body = parse_json(&assert.get_output().stderr);
    assert_eq!(body["error"], "plugins-refresh-consent-required");

    assert!(
        cursor.path().join("plugins/cache/augentic").exists(),
        "a refused refresh must not delete the cache"
    );
}
