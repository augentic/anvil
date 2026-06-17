//! Acceptance matrix for the three `specify init` shapes:
//! `greenfield`, `brownfield`, and `workspace`.
//!
//! Each test drives the real `specify` binary over a throwaway tempdir
//! and asserts the on-disk + JSON-envelope contract for one shape:
//!
//! - `greenfield` — a fresh `specify init <adapter>` over an empty dir
//!   scaffolds `.specify/` and pins the current `specify_version`.
//! - `brownfield` — `specify init --upgrade` over a populated regular
//!   project bumps only the pin, keeps operator artifacts byte-stable,
//!   and re-runs as a no-op.
//! - `workspace` — the same re-entry over a populated workspace,
//!   with the `workspace: true` discriminator and `registry.yaml` preserved.
//!
//! The `brownfield` / `workspace` headline invariants are also covered with an
//! exhaustive byte-level write-set diff by
//! `init_upgrade_bumps_only_version_and_preserves_artifacts` and
//! `init_upgrade_preserves_workspace_and_registry` in `tests/init.rs`; the
//! versions here keep all three shapes co-located as one readable matrix.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use specify_workflow::config::ProjectConfig;
use tempfile::tempdir;

use crate::common::{omnia_schema_dir, parse_json, specify_cmd};

/// Version this binary stamps into `specify_version` (the `specify`
/// crate and this test crate share the workspace version).
const BINARY_VERSION: &str = env!("CARGO_PKG_VERSION");

// ---- greenfield ----

#[test]
fn greenfield() {
    let tmp = tempdir().unwrap();
    let assert = specify_cmd()
        .current_dir(tmp.path())
        .args(["--format", "json", "init"])
        .arg(omnia_schema_dir())
        .args(["--name", "greenfield-proj"])
        .assert()
        .success();
    let body = parse_json(&assert.get_output().stdout);
    assert_eq!(body["adapter-name"], "omnia");
    assert_eq!(body["specify-version"], BINARY_VERSION);
    assert_eq!(body["specify-version-changed"], true);

    // Fresh init scaffolds the canonical `.specify/` skeleton. The cache
    // is regenerable, machine-owned state and lives out-of-tree, so it is
    // no longer part of the in-tree skeleton.
    for dir in [".specify", ".specify/slices", ".specify/specs", ".specify/archive"] {
        assert!(tmp.path().join(dir).is_dir(), "greenfield must scaffold {dir}");
    }

    let cfg = load_cfg(tmp.path());
    assert_eq!(cfg.specify_version.as_deref(), Some(BINARY_VERSION));
    // `adapter:` persists the resolved source value (the fixture URI),
    // while the JSON envelope above carries the resolved name `omnia`.
    assert!(
        cfg.adapter.as_deref().is_some_and(|value| value.contains("omnia")),
        "greenfield must persist the omnia adapter, got {:?}",
        cfg.adapter,
    );
    assert!(!cfg.workspace, "greenfield must not write the workspace discriminator");
}

// ---- brownfield ----

#[test]
fn brownfield() {
    // Concise matrix view of the regular re-entry upgrade. Exhaustive
    // write-set coverage: tests/init.rs::
    // init_upgrade_bumps_only_version_and_preserves_artifacts (Change E).
    let tmp = tempdir().unwrap();
    let specify = tmp.path().join(".specify");
    fs::create_dir_all(specify.join("slices/my-slice")).unwrap();
    fs::write(
        specify.join("project.yaml"),
        "name: brownfield\nadapter: omnia\nspecify_version: 0.1.0\nrules:\n  specs: specs.md\n",
    )
    .unwrap();
    fs::write(specify.join("slices/my-slice/spec.md"), "# operator slice\n").unwrap();
    fs::write(tmp.path().join("AGENTS.md"), "# operator AGENTS.md\n").unwrap();

    let before = snapshot(tmp.path());
    let assert = specify_cmd()
        .current_dir(tmp.path())
        .args(["--format", "json", "init", "--upgrade"])
        .assert()
        .success();
    let body = parse_json(&assert.get_output().stdout);
    assert_eq!(body["specify-version"], BINARY_VERSION);
    assert_eq!(body["specify-version-changed"], true);
    assert_eq!(body["adapter-name"], "omnia");

    assert_only_project_yaml_changed(&before, &snapshot(tmp.path()));
    let cfg = load_cfg(tmp.path());
    assert_eq!(cfg.specify_version.as_deref(), Some(BINARY_VERSION));
    assert_eq!(cfg.adapter.as_deref(), Some("omnia"));

    assert_second_run_is_noop(tmp.path());
}

// ---- workspace ----

#[test]
fn workspace() {
    // Concise matrix view of the workspace re-entry upgrade. Exhaustive
    // coverage: tests/init.rs::init_upgrade_preserves_workspace_and_registry
    // (Change E).
    let tmp = tempdir().unwrap();
    let specify = tmp.path().join(".specify");
    fs::create_dir_all(&specify).unwrap();
    fs::write(
        specify.join("project.yaml"),
        "name: platform-workspace\nspecify_version: 0.1.0\nworkspace: true\n",
    )
    .unwrap();
    fs::write(tmp.path().join("registry.yaml"), "version: 1\nprojects: []\n").unwrap();
    fs::write(tmp.path().join("AGENTS.md"), "# workspace sentinel\n").unwrap();

    let before = snapshot(tmp.path());
    let assert = specify_cmd()
        .current_dir(tmp.path())
        .args(["--format", "json", "init", "--upgrade"])
        .assert()
        .success();
    let body = parse_json(&assert.get_output().stdout);
    assert_eq!(body["specify-version"], BINARY_VERSION);
    assert_eq!(body["specify-version-changed"], true);
    assert_eq!(body["adapter-name"], "workspace");

    assert_only_project_yaml_changed(&before, &snapshot(tmp.path()));
    let cfg = load_cfg(tmp.path());
    assert!(cfg.workspace, "workspace discriminator must survive the upgrade");
    assert!(cfg.adapter.is_none(), "workspace upgrade must not synthesise an adapter");
    assert_eq!(cfg.specify_version.as_deref(), Some(BINARY_VERSION));

    assert_second_run_is_noop(tmp.path());
}

// ---- helpers ----

/// Parse `.specify/project.yaml` under `root` into a [`ProjectConfig`].
fn load_cfg(root: &Path) -> ProjectConfig {
    let text = fs::read_to_string(root.join(".specify/project.yaml")).expect("read project.yaml");
    serde_saphyr::from_str(&text).expect("parse project.yaml")
}

/// Snapshot every regular file under `root` as `relative-path -> bytes`.
fn snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(dir).expect("read_dir") {
            let entry = entry.expect("dir entry");
            let path = entry.path();
            let rel = path.strip_prefix(root).expect("strip prefix").to_path_buf();
            if entry.file_type().expect("file_type").is_dir() {
                walk(root, &path, out);
            } else {
                out.insert(rel, fs::read(&path).expect("read file"));
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

/// Assert the only path that differs between two tree snapshots is
/// `.specify/project.yaml` — the closed write set of `init --upgrade`.
fn assert_only_project_yaml_changed(
    before: &BTreeMap<PathBuf, Vec<u8>>, after: &BTreeMap<PathBuf, Vec<u8>>,
) {
    let project_yaml = PathBuf::from(".specify/project.yaml");
    let before_keys: Vec<_> = before.keys().filter(|k| **k != project_yaml).collect();
    let after_keys: Vec<_> = after.keys().filter(|k| **k != project_yaml).collect();
    assert_eq!(before_keys, after_keys, "upgrade must not add or remove files");
    for key in before_keys {
        assert_eq!(before[key], after[key], "file {} must be byte-identical", key.display());
    }
}

/// Run a second `init --upgrade` over `root` and assert it is a
/// byte-stable no-op (`specify-version-changed: false`, tree unchanged).
fn assert_second_run_is_noop(root: &Path) {
    let before = snapshot(root);
    let assert = specify_cmd()
        .current_dir(root)
        .args(["--format", "json", "init", "--upgrade"])
        .assert()
        .success();
    assert_eq!(
        parse_json(&assert.get_output().stdout)["specify-version-changed"],
        false,
        "second --upgrade must be a no-op",
    );
    assert_eq!(snapshot(root), before, "second --upgrade must leave the tree byte-identical");
}
