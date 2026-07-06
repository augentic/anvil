//! Engine/schema invariant guards: the lint engine carries no rule
//! policy, and every accepted hint kind is executable (no reserved
//! kinds, schema and interpreter set stay in lockstep).

mod no_embedded_policy {
    //! Layer-3 verification guard: no rule policy in the lint engine.
    //! (See DECISIONS.md §"Framework lint engine: generic dispatcher (Road A / Road B)".)
    //!
    //! Proves no rule policy lives baked into the lint engine. It scans the
    //! deterministic hint eval arms (`lint/eval/`) and FAILS if any
    //! rule-specific policy literal reappears:
    //!
    //! - a value-bearing discriminator (`*-equals-v1`,
    //!   `adapter-manifest-version-*`),
    //! - an owner->prefix policy map (`BUILTIN_NAMESPACES` / `TARGET_OWNERS`),
    //! - a canonical-document path string, or
    //! - a numeric cap `const` whose name is not on the mechanism allow-list.
    //!
    //! Every rule-specific value must instead ride the rule's `config:` (in
    //! the `specify` repo). The only engine-side constants this guard
    //! tolerates are mechanism — evidence/snippet/iteration bounds —
    //! enumerated by name in [`MECHANISM_CAP_CONSTS`] with the reason each
    //! is mechanism, not policy.

    use std::path::{Path, PathBuf};

    use regex::Regex;

    /// Numeric cap `const`s that are mechanism, not rule policy, and are
    /// therefore exempt from the "no bare numeric cap" guard. Each is keyed
    /// by const name with the reason it is mechanism.
    const MECHANISM_CAP_CONSTS: &[(&str, &str)] = &[
        ("STDERR_MAX_BYTES", "tool stderr truncation budget (wire mechanism)"),
        ("SNIPPET_MAX_CHARS", "evidence snippet truncation budget (finding mechanism)"),
        ("CLAMP_ITERATION_LIMIT", "evidence-size clamp loop bound (finding mechanism)"),
    ];

    /// Literal substrings that only ever appear as relocated rule policy.
    /// Any reappearance in the scanned engine source is a Layer-3 regression.
    const FORBIDDEN_SUBSTRINGS: &[(&str, &str)] = &[
        ("equals-v1", "constant-eq version discriminator (CORE-006) — use config"),
        ("adapter-manifest-version", "constant-eq version discriminator (CORE-006) — use config"),
        ("BUILTIN_NAMESPACES", "namespace owner->prefix policy map (CORE-009) — use config"),
        ("TARGET_OWNERS", "namespace owner->prefix policy map (CORE-009) — use config"),
    ];

    fn crate_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    /// The engine source roots policy could hide in: the hint eval arms.
    fn scan_roots() -> Vec<PathBuf> {
        let root = crate_root();
        vec![root.join("src/lint/eval")]
    }

    fn collect_rs_files(path: &Path, out: &mut Vec<PathBuf>) {
        if path.is_file() {
            if path.extension().is_some_and(|ext| ext == "rs") {
                out.push(path.to_path_buf());
            }
            return;
        }
        let Ok(entries) = std::fs::read_dir(path) else { return };
        for entry in entries.flatten() {
            collect_rs_files(&entry.path(), out);
        }
    }

    fn scanned_files() -> Vec<PathBuf> {
        let mut files = Vec::new();
        for root in scan_roots() {
            collect_rs_files(&root, &mut files);
        }
        assert!(!files.is_empty(), "guard scanned zero files — scan roots drifted");
        files.sort();
        files
    }

    fn rel(path: &Path) -> String {
        path.strip_prefix(crate_root()).unwrap_or(path).to_string_lossy().into_owned()
    }

    #[test]
    fn no_value_bearing_discriminators() {
        for file in scanned_files() {
            let content = std::fs::read_to_string(&file).expect("read scanned file");
            for (needle, reason) in FORBIDDEN_SUBSTRINGS {
                assert!(
                    !content.contains(needle),
                    "{}: forbidden rule policy literal `{needle}` reappeared ({reason}). \
                 Move the value into the rule's `config:` in the specify repo.",
                    rel(&file),
                );
            }
        }
    }

    #[test]
    fn no_unallowlisted_numeric_caps() {
        let cap_const = Regex::new(r"const\s+([A-Z][A-Z0-9_]*)\s*:\s*[A-Za-z0-9_]+\s*=\s*[0-9]")
            .expect("cap-const regex compiles");
        for file in scanned_files() {
            let content = std::fs::read_to_string(&file).expect("read scanned file");
            for caps in cap_const.captures_iter(&content) {
                let name = &caps[1];
                let looks_like_cap =
                    name.contains("MAX") || name.contains("LIMIT") || name.contains("CAP");
                if !looks_like_cap {
                    continue;
                }
                assert!(
                    MECHANISM_CAP_CONSTS.iter().any(|(allowed, _)| *allowed == name),
                    "{}: numeric cap `const {name}` is not on the mechanism allow-list. A \
                 rule-specific cap must ride the rule's `config: {{ max }}` in the specify \
                 repo; if this is genuinely mechanism, add it to MECHANISM_CAP_CONSTS with a \
                 reason.",
                    rel(&file),
                );
            }
        }
    }
}

mod no_reserved_hint_kinds {
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
        let schema: Value = serde_json::from_str(specify_schema::RULE_JSON_SCHEMA)
            .expect("rule.schema.json parses");
        assert_no_reserved(&schema);
    }

    #[test]
    fn every_kind_has_interpreter() {
        let schema: Value = serde_json::from_str(specify_schema::RULE_JSON_SCHEMA)
            .expect("rule.schema.json parses");
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
        let schema: Value = serde_json::from_str(specify_schema::RULE_JSON_SCHEMA)
            .expect("rule.schema.json parses");
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
}
