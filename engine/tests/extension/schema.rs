//! Integration tests for `specify tool schema`.

use std::fs;
use std::path::PathBuf;

use tempfile::tempdir;

use crate::common::{parse_json, repo_root, scaffold_tool_project, specify_cmd};

// Adapter tool wasm now lives with each adapter in
// `augentic/specify-adapters` (committed `adapter.wasm`), built by that
// repo's CI. These vectis cases self-skip when the wasm is absent — the
// happy-path acceptance coverage runs in the adapters repo.
fn vectis_wasm() -> PathBuf {
    repo_root().join("target/vectis-wasi-tools/release/vectis.wasm")
}

#[test]
fn schema_vectis_tokens_returns_valid_json() {
    let wasm = vectis_wasm();
    if !wasm.is_file() {
        eprintln!(
            "skipping: vectis WASM not found at {}; run `cargo make vectis-wasm`",
            wasm.display()
        );
        return;
    }

    let tmp = tempdir().expect("tempdir");
    let (project, cache) = scaffold_tool_project(&tmp, "vectis", &wasm);

    let assert = specify_cmd()
        .current_dir(&project)
        .env("SPECIFY_EXTENSIONS_CACHE", &cache)
        .args(["extension", "schema", "vectis", "tokens"])
        .assert()
        .success();

    let value = parse_json(&assert.get_output().stdout);
    assert!(value.is_object(), "output must be a JSON object");
    let id = value["$id"].as_str().expect("$id field must be present");
    assert!(id.contains("tokens"), "$id should reference 'tokens', got: {id}");
}

#[test]
fn schema_vectis_unknown_name_exits_nonzero() {
    let wasm = vectis_wasm();
    if !wasm.is_file() {
        eprintln!(
            "skipping: vectis WASM not found at {}; run `cargo make vectis-wasm`",
            wasm.display()
        );
        return;
    }

    let tmp = tempdir().expect("tempdir");
    let (project, cache) = scaffold_tool_project(&tmp, "vectis", &wasm);

    let assert = specify_cmd()
        .current_dir(&project)
        .env("SPECIFY_EXTENSIONS_CACHE", &cache)
        .args(["extension", "schema", "vectis", "nonexistent"])
        .assert()
        .failure();

    let value = parse_json(&assert.get_output().stdout);
    assert_eq!(value["error"], "unknown-schema");
    assert_eq!(value["exit-code"], 2);
}

#[test]
fn schema_undeclared_tool_exits_two() {
    let tmp = tempdir().expect("tempdir");
    let project = tmp.path();
    fs::create_dir_all(project.join(".specify")).expect("create .specify");
    fs::write(
        project.join(".specify/project.yaml"),
        "name: schema-test\nworkspace: true\ntools: []\n",
    )
    .expect("write project.yaml");

    let cache =
        std::env::temp_dir().join(format!("specify-tool-schema-undeclared-{}", std::process::id()));
    fs::create_dir_all(&cache).expect("create cache");

    let assert = specify_cmd()
        .current_dir(project)
        .env("SPECIFY_EXTENSIONS_CACHE", &cache)
        .args(["--format", "json", "extension", "schema", "nosuch", "tokens"])
        .assert()
        .failure();

    assert_eq!(assert.get_output().status.code(), Some(2));
    let value = parse_json(&assert.get_output().stderr);
    assert_eq!(value["error"], "tool-not-declared");
}

// `schema` membership in the `tool` verb inventory is asserted by
// `run::help_lists_active_verbs` against the contract dump.
