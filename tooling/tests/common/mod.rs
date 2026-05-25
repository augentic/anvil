//! Shared helpers for cross-repo acceptance tests (`tooling/tests/*`).
//!
//! Cargo treats every `tooling/tests/*.rs` file as a separate integration-test
//! binary, but skips subdirectories. The canonical pattern for sharing code
//! between those binaries is `tests/<helper>/mod.rs` — see
//! `docs/standards/coding-standards.md` (Module layout) for the local rule.
//!
//! Each integration test that uses these helpers declares `mod common;` at
//! crate root. Items marked `pub` are visible to that one test binary only,
//! so `#![allow(dead_code)]` here avoids per-binary unused-symbol warnings.

// Each `tests/<name>.rs` mounts this module independently and only consumes
// part of the toolbox. Suppress per-binary unused warnings for the rest.
#![allow(dead_code, unused_imports)]

pub mod fixtures;
pub mod golden;
pub mod schema;
pub mod specify;

pub use fixtures::{
    walk_skill_fixtures, walk_source_fixtures, walk_target_fixtures, SkillFixture, SourceFixture,
    TargetFixture,
};
pub use golden::{assert_golden, assert_golden_tree, regenerate_goldens};
pub use schema::{validate_cli_schema_or_skip, validate_yaml_file_or_skip, CliSchemaId};
pub use specify::{resolve_specify_bin, skip_unless_specify_bin, with_specify_bin, SpecifyResult};

use std::path::Path;

use tooling::Context;

/// Framework context resolved from the tooling crate manifest directory.
pub fn framework_context() -> Context {
    Context::from_manifest_dir(env!("CARGO_MANIFEST_DIR")).expect("framework root resolves")
}

/// `tests/fixtures/` under the framework root.
pub fn fixtures_dir(ctx: &Context) -> std::path::PathBuf {
    ctx.framework_root().join("tests").join("fixtures")
}

/// Read a UTF-8 text file; `None` when absent.
pub fn read_text(path: impl AsRef<Path>) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

/// Parse YAML into a JSON value via `serde_saphyr`.
pub fn read_yaml(path: impl AsRef<Path>) -> Result<serde_json::Value, String> {
    let raw = std::fs::read_to_string(path.as_ref())
        .map_err(|err| format!("read {}: {err}", path.as_ref().display()))?;
    serde_saphyr::from_str(&raw).map_err(|err| format!("YAML parse {}: {err}", path.as_ref().display()))
}

/// Non-empty markdown with at least one `#` heading line.
pub fn assert_non_empty_markdown(path: impl AsRef<Path>) -> Result<(), String> {
    let path = path.as_ref();
    let content = read_text(path).ok_or_else(|| format!("{}: missing", path.display()))?;
    if content.trim().is_empty() {
        return Err(format!("{}: empty", path.display()));
    }
    if !content.lines().any(|line| line.starts_with('#')) {
        return Err(format!("{}: no markdown heading", path.display()));
    }
    Ok(())
}

/// Parse YAML and require a top-level mapping (not array or scalar).
pub fn assert_yaml_mapping(path: impl AsRef<Path>) -> Result<serde_json::Value, String> {
    let path = path.as_ref();
    let data = read_yaml(path)?;
    if !data.is_object() {
        return Err(format!("{}: expected a YAML mapping", path.display()));
    }
    Ok(data)
}

/// Walk a directory tree yielding every `.yaml` file path.
pub fn walk_yaml(root: impl AsRef<Path>) -> Vec<std::path::PathBuf> {
    let root = root.as_ref();
    if !root.is_dir() {
        return Vec::new();
    }
    walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .is_some_and(|ext| ext == "yaml" || ext == "yml")
        })
        .map(|entry| entry.into_path())
        .collect()
}
