//! C18 closer regression test: every deterministic-hint kind the
//! authoring schema accepts is executable, and the schema carries no
//! `x-hint-status: reserved` annotations.
//!
//! The reserved hint kinds land one interpreter at a time.
//! Once the last kind ships, no `const` in the
//! `hints[].kind` `oneOf` may carry `"x-hint-status": "reserved"`, and
//! every kind must have a matching `src/lint/eval/<kind>.rs`
//! interpreter module. This test is cheap insurance against the schema
//! and the interpreter set drifting apart again: adding a new kind to
//! `rule.schema.json` without an interpreter (or re-introducing a
//! `reserved` annotation) fails here rather than silently skipping at
//! evaluation time.

use std::collections::BTreeSet;
use std::path::Path;

use serde_json::Value;

/// Directory holding one `<kind>.rs` interpreter module per executable
/// hint kind, relative to this crate's manifest.
const EVAL_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src/lint/eval");

/// Recursively assert no object anywhere in the schema carries a
/// `"x-hint-status": "reserved"` annotation.
fn assert_no_reserved(node: &Value) {
    match node {
        Value::Object(map) => {
            if let Some(status) = map.get("x-hint-status") {
                assert_ne!(
                    status.as_str(),
                    Some("reserved"),
                    "rule.schema.json still carries an `x-hint-status: reserved` annotation; \
                     every reserved hint kind must ship an interpreter",
                );
            }
            for value in map.values() {
                assert_no_reserved(value);
            }
        }
        Value::Array(items) => {
            for value in items {
                assert_no_reserved(value);
            }
        }
        _ => {}
    }
}

/// Recursively collect every `const` string value. `rule.schema.json`
/// uses `const` exclusively for the `hints[].kind` enum, so the
/// collected set is precisely the accepted hint-kind vocabulary.
fn collect_const_strings(node: &Value, out: &mut BTreeSet<String>) {
    match node {
        Value::Object(map) => {
            if let Some(Value::String(s)) = map.get("const") {
                out.insert(s.clone());
            }
            for value in map.values() {
                collect_const_strings(value, out);
            }
        }
        Value::Array(items) => {
            for value in items {
                collect_const_strings(value, out);
            }
        }
        _ => {}
    }
}

/// Map a kebab-case hint kind to its `snake_case` interpreter module
/// file name (`reference-resolves` -> `reference_resolves.rs`).
fn module_file(kind: &str) -> String {
    format!("{}.rs", kind.replace('-', "_"))
}

#[test]
fn schema_carries_no_reserved_hint_kinds() {
    let schema: Value =
        serde_json::from_str(specify_schema::RULE_JSON_SCHEMA).expect("rule.schema.json parses");
    assert_no_reserved(&schema);
}

#[test]
fn every_kind_has_interpreter() {
    let schema: Value =
        serde_json::from_str(specify_schema::RULE_JSON_SCHEMA).expect("rule.schema.json parses");
    let mut kinds = BTreeSet::new();
    collect_const_strings(&schema, &mut kinds);

    assert!(
        kinds.len() >= 12,
        "expected the full v1 hint-kind vocabulary in rule.schema.json, found {}: {:?}",
        kinds.len(),
        kinds,
    );

    let eval_dir = Path::new(EVAL_DIR);
    for kind in &kinds {
        let module = eval_dir.join(module_file(kind));
        assert!(
            module.exists(),
            "hint kind `{}` is accepted by rule.schema.json but has no interpreter module at {}",
            kind,
            module.display(),
        );
    }
}

#[test]
fn every_interpreter_maps_to_kind() {
    let schema: Value =
        serde_json::from_str(specify_schema::RULE_JSON_SCHEMA).expect("rule.schema.json parses");
    let mut kinds = BTreeSet::new();
    collect_const_strings(&schema, &mut kinds);
    let module_files: BTreeSet<String> = kinds.iter().map(|k| module_file(k)).collect();

    for entry in std::fs::read_dir(EVAL_DIR).expect("eval dir is readable") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        if path.extension().is_none_or(|ext| !ext.eq_ignore_ascii_case("rs")) {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        // `error.rs` (the `HintError` enum), `finding.rs` (the finding
        // constructors + evidence-cap clamp, with its own unit tests), and
        // `testkit.rs` (the cfg(test) unit-test scaffolding) are
        // orchestrator-support submodules of `eval.rs`, not hint-kind
        // interpreters; skip them.
        if matches!(name.as_str(), "error.rs" | "finding.rs" | "testkit.rs") {
            continue;
        }
        assert!(
            module_files.contains(&name),
            "interpreter module `{name}` has no matching hint kind in rule.schema.json; \
             add the `const` or remove the orphan module",
        );
    }
}
