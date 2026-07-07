//! Integration tests for the `specify rules` surface (`export` and the
//! shared codex distribution). Shared helpers live in [`common`].

mod codex {
    //! Integration tests for shared codex materialization (RM-07 /
    //! DECISIONS.md §"Codex ownership flip: shared packs embed in the
    //! binary").
    //!
    //! Cover `specify init` materializing the binary-embedded packs into
    //! the project codex cache, `specify rules sync` refreshing them
    //! (version re-pin, `--include-framework`), and `specify rules
    //! export` resolving the materialized shared rules without a
    //! `--rules-root` flag.

    use std::fs;
    use std::path::Path;

    use tempfile::tempdir;

    use crate::common::{expected_cache_dir, omnia_component, parse_json, specify_cmd};

    /// A shared rule id known to ship in the embedded `universal/`
    /// pack (`codex/rules/universal/dead-code.md`).
    const EMBEDDED_RULE_ID: &str = "UNI-013";
    /// Cache-relative path of that rule after materialization.
    const EMBEDDED_RULE_REL: &str = "codex/codex/rules/universal/dead-code.md";

    fn init_project(project: &Path) {
        specify_cmd()
            .current_dir(project)
            .args(["init"])
            .arg(omnia_component())
            .args(["--name", "demo"])
            .assert()
            .success();
    }

    #[test]
    fn init_materializes_embedded_codex() {
        let project = tempdir().unwrap();
        init_project(project.path());

        let cache = expected_cache_dir(project.path());
        assert!(
            cache.join(EMBEDDED_RULE_REL).is_file(),
            "init must materialize the embedded shared codex into the cache"
        );
        assert!(
            !cache.join("codex/codex/rules/core").exists(),
            "the framework core pack must not materialize without --include-framework"
        );
        let meta = fs::read_to_string(cache.join("codex/codex-meta.yaml")).expect("codex meta");
        assert!(
            meta.contains(env!("CARGO_PKG_VERSION")),
            "codex-meta.yaml must pin the binary version, got:\n{meta}"
        );

        // `rules export` resolves the materialized shared rule with NO
        // `--rules-root`: the resolver picks up the codex cache rung.
        let assert = specify_cmd()
            .current_dir(project.path())
            .args(["--format", "json", "rules", "export", "--target", "omnia"])
            .assert()
            .success();
        let value = parse_json(&assert.get_output().stdout);
        let rules = value["rules"].as_array().expect("rules array in export envelope");
        assert!(
            rules.iter().any(|r| r["rule-id"] == EMBEDDED_RULE_ID),
            "exported codex must carry the embedded {EMBEDDED_RULE_ID} rule, got:\n{value:#}"
        );
    }

    #[test]
    fn rules_sync_refreshes_codex_cache() {
        let project = tempdir().unwrap();
        init_project(project.path());

        // Drop the materialized cache, then refresh it via `rules sync`
        // (adapter-independent: the packs come from the binary).
        fs::remove_dir_all(expected_cache_dir(project.path()).join("codex"))
            .expect("rm codex cache");

        let assert = specify_cmd()
            .current_dir(project.path())
            .args(["--format", "json", "rules", "sync"])
            .assert()
            .success();
        let value = parse_json(&assert.get_output().stdout);
        assert_eq!(value["distributed"], true, "sync must rematerialize, got:\n{value:#}");
        assert_eq!(
            value["source"],
            env!("CARGO_PKG_VERSION"),
            "sync must pin the binary version, got:\n{value:#}"
        );

        let cached = expected_cache_dir(project.path()).join(EMBEDDED_RULE_REL);
        assert!(cached.is_file(), "rules sync must repopulate the codex cache");
    }

    #[test]
    fn rules_sync_rematerializes_on_drift() {
        let project = tempdir().unwrap();
        init_project(project.path());
        let cache = expected_cache_dir(project.path());

        // `--include-framework` differs from the recorded stamp, so sync
        // rewrites the cache and adds the core pack.
        specify_cmd()
            .current_dir(project.path())
            .args(["rules", "sync", "--include-framework"])
            .assert()
            .success();
        assert!(
            cache.join("codex/codex/rules/core").is_dir(),
            "--include-framework must materialize the core pack"
        );

        // A stamp from an older binary re-materializes: simulate one by
        // rewriting the recorded version, then deleting a rule file.
        let meta_path = cache.join("codex/codex-meta.yaml");
        let stale = fs::read_to_string(&meta_path)
            .expect("codex meta")
            .replace(env!("CARGO_PKG_VERSION"), "0.0.0");
        fs::write(&meta_path, stale).expect("write stale meta");
        fs::remove_file(cache.join(EMBEDDED_RULE_REL)).expect("rm embedded rule");

        specify_cmd().current_dir(project.path()).args(["rules", "sync"]).assert().success();
        assert!(
            cache.join(EMBEDDED_RULE_REL).is_file(),
            "a stale binary-version stamp must trigger re-materialization"
        );
    }

    #[test]
    fn rules_sync_on_workspace() {
        let tmp = tempdir().unwrap();
        specify_cmd()
            .current_dir(tmp.path())
            .args(["init", "--name", "platform-workspace", "--workspace"])
            .assert()
            .success();

        // Adapter-independent materialization works on adapter-less
        // workspace projects too.
        specify_cmd().current_dir(tmp.path()).args(["rules", "sync"]).assert().success();
        assert!(
            expected_cache_dir(tmp.path()).join(EMBEDDED_RULE_REL).is_file(),
            "workspace `rules sync` must materialize the embedded codex"
        );
    }
}

mod export {
    //! Tests for `specify rules export`.
    //!
    //! Exercises the runtime rules export contract — `ResolvedRules` export
    //! and §"Codex root resolution (v1)" — via the
    //! [`specify_standards::build_resolved_rules`] library entrypoint for the
    //! resolver scenarios and `assert_cmd` for the CLI-plumbing scenarios
    //! (`--include-core`, JSON-only, `rules-root-required`).
    //!
    //! Every scenario is self-contained: it builds a synthetic rules-root
    //! tree under a tempdir, so the suite has no dependency on the sibling
    //! `augentic/specify` checkout. Rule-*content* validation (e.g. the
    //! `## Rule` body heading) is enforced separately by the framework lint
    //! (CORE-053), run over the plugin repo by its own `make lint`.

    use std::fs;
    use std::path::Path;

    use assert_cmd::Command;
    use serde_json::Value;
    use specify_standards::{Origin, ResolveInputs, ResolvedRules, build_resolved_rules};
    use tempfile::tempdir;

    /// Build the export envelope by calling the library entrypoint directly
    /// against an explicit synthetic rules root and project dir.
    fn resolve_rules(
        rules_root: &Path, project: &Path, target: &str, sources: &[String],
        include_deprecated: bool,
    ) -> ResolvedRules {
        let inputs = ResolveInputs {
            project_dir: project,
            rules_root: Some(rules_root),
            target_adapter: target,
            source_adapters: sources,
            artifact_paths: &[],
            languages: &[],
            include_deprecated,
            include_unmatched: false,
            include_core: false,
        };
        build_resolved_rules(&inputs).expect("build_resolved_rules succeeds")
    }

    /// Write arbitrary rule markdown, creating parent dirs as needed.
    fn write_rule_md(path: &Path, contents: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create parent dir");
        }
        fs::write(path, contents).expect("write rule fixture");
    }

    /// Minimal frontmatter + body that parses through the CH-11 parser and
    /// validates against `rule.schema.json`.
    fn basic_rule(id: &str, title: &str, severity: &str) -> String {
        format!(
            "---\nid: {id}\ntitle: {title}\nseverity: {severity}\ntrigger: Synthetic resolver fixture trigger sentence long enough for schema.\n---\n\n## Rule\n\nBody for {id}.\n"
        )
    }

    /// A deprecated rule. Authored with the `snake_case` `replaced_by` key
    /// (the CH-11 parser lifts it to the kebab-case `replaced-by` wire form);
    /// schema validation runs on the pre-lift YAML.
    fn deprecated_rule(id: &str, replaced_by: &str) -> String {
        format!(
            "---\nid: {id}\ntitle: Deprecated fixture\nseverity: important\ntrigger: Synthetic deprecated fixture trigger sentence long enough for schema.\ndeprecated:\n  reason: superseded by a fixture replacement\n  replaced_by: {replaced_by}\n---\n\n## Rule\n\nBody for {id}.\n"
        )
    }

    /// Target overlay rolls up alongside the shared `UNI-*` pack, each with
    /// the contract origin tier.
    #[test]
    fn target_overlay_rolls_up_with_shared() {
        let rules_root = tempdir().expect("rules root");
        let project = tempdir().expect("project");
        write_rule_md(
            &rules_root.path().join("codex/rules/universal/uni-001.md"),
            &basic_rule("UNI-001", "Shared anchor", "important"),
        );
        write_rule_md(
            &project.path().join("adapters/targets/omnia/prose/rules/omnia-001.md"),
            &basic_rule("OMNIA-001", "Target overlay", "important"),
        );

        let resolved = resolve_rules(rules_root.path(), project.path(), "omnia", &[], false);

        let uni = resolved.rules.iter().find(|r| r.rule_id == "UNI-001").expect("UNI-001 present");
        let omnia =
            resolved.rules.iter().find(|r| r.rule_id == "OMNIA-001").expect("OMNIA-001 present");
        assert_eq!(uni.origin, Origin::Shared, "UNI-001 must carry origin=shared");
        assert_eq!(omnia.origin, Origin::Target, "OMNIA-001 must carry origin=target");
    }

    /// A source-adapter overlay flows in as `origin: source` when the source
    /// is bound to the export context.
    #[test]
    fn source_overlay_carries_origin_source() {
        let rules_root = tempdir().expect("rules root");
        let project = tempdir().expect("project");
        write_rule_md(
            &rules_root.path().join("codex/rules/universal/uni-001.md"),
            &basic_rule("UNI-001", "Shared anchor", "important"),
        );
        write_rule_md(
            &project.path().join("adapters/sources/documentation/prose/rules/src-001.md"),
            &basic_rule("SRC-001", "Source overlay", "important"),
        );

        let sources = vec!["documentation".to_string()];
        let resolved = resolve_rules(rules_root.path(), project.path(), "omnia", &sources, false);

        let src = resolved
            .rules
            .iter()
            .find(|r| r.rule_id == "SRC-001")
            .expect("SRC-001 must appear when documentation source is bound");
        assert_eq!(src.origin, Origin::Source, "SRC-001 must carry origin=source");
    }

    /// `--include-deprecated` toggles the deprecation filter: deprecated
    /// rules are dropped by default and surface only with the flag set.
    #[test]
    fn include_deprecated_surfaces_rule() {
        let rules_root = tempdir().expect("rules root");
        let project = tempdir().expect("project");
        write_rule_md(
            &rules_root.path().join("codex/rules/universal/uni-001.md"),
            &basic_rule("UNI-001", "Active shared", "important"),
        );
        write_rule_md(
            &rules_root.path().join("codex/rules/universal/uni-009.md"),
            &deprecated_rule("UNI-009", "UNI-001"),
        );

        let without = resolve_rules(rules_root.path(), project.path(), "omnia", &[], false);
        assert!(
            !without.rules.iter().any(|r| r.rule_id == "UNI-009"),
            "deprecated UNI-009 must be filtered out without --include-deprecated",
        );

        let with = resolve_rules(rules_root.path(), project.path(), "omnia", &[], true);
        assert!(
            with.rules.iter().any(|r| r.rule_id == "UNI-009"),
            "deprecated UNI-009 must surface with --include-deprecated",
        );
    }

    /// The deprecation successor serialises with the kebab-case `replaced-by`
    /// wire key; the `snake_case` spelling never appears in the envelope.
    #[test]
    fn replaced_by_uses_kebab_wire_key() {
        let rules_root = tempdir().expect("rules root");
        let project = tempdir().expect("project");
        write_rule_md(
            &rules_root.path().join("codex/rules/universal/uni-001.md"),
            &basic_rule("UNI-001", "Active shared", "important"),
        );
        write_rule_md(
            &rules_root.path().join("codex/rules/universal/uni-009.md"),
            &deprecated_rule("UNI-009", "UNI-001"),
        );

        let resolved = resolve_rules(rules_root.path(), project.path(), "omnia", &[], true);
        let body = serde_json::to_string(&resolved).expect("serialise");
        assert!(
            body.contains("\"replaced-by\""),
            "kebab-case `replaced-by` wire key must be present"
        );
        assert!(
            !body.contains("\"replaced_by\""),
            "snake_case wire key `replaced_by` must not appear in the export envelope",
        );
    }

    /// Every wire `path` is anchored (no leading `/`, no Windows drive
    /// prefix, no backslash separators) — no absolute machine path leaks.
    #[test]
    fn paths_anchored_not_absolute() {
        let rules_root = tempdir().expect("rules root");
        let project = tempdir().expect("project");
        write_rule_md(
            &rules_root.path().join("codex/rules/universal/uni-001.md"),
            &basic_rule("UNI-001", "Shared anchor", "important"),
        );
        write_rule_md(
            &project.path().join("adapters/targets/omnia/prose/rules/omnia-001.md"),
            &basic_rule("OMNIA-001", "Target overlay", "important"),
        );

        let resolved = resolve_rules(rules_root.path(), project.path(), "omnia", &[], false);
        for rule in &resolved.rules {
            assert!(
                !rule.path.starts_with('/'),
                "rule {} path leaked an absolute prefix: {}",
                rule.rule_id,
                rule.path,
            );
            let bytes = rule.path.as_bytes();
            let drive_letter =
                bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':';
            assert!(
                !drive_letter,
                "rule {} path leaked a Windows drive prefix: {}",
                rule.rule_id, rule.path,
            );
            assert!(
                !rule.path.contains('\\'),
                "rule {} path leaked a backslash separator: {}",
                rule.rule_id,
                rule.path,
            );
        }
    }

    /// Two back-to-back exports with the same inputs emit byte-identical
    /// JSON — pins determinism at the `serde_json` boundary.
    #[test]
    fn stable_ordering_byte_identical() {
        let rules_root = tempdir().expect("rules root");
        let project = tempdir().expect("project");
        write_rule_md(
            &rules_root.path().join("codex/rules/universal/uni-001.md"),
            &basic_rule("UNI-001", "Critical shared", "critical"),
        );
        write_rule_md(
            &rules_root.path().join("codex/rules/universal/uni-002.md"),
            &basic_rule("UNI-002", "Optional shared", "optional"),
        );
        write_rule_md(
            &project.path().join("adapters/targets/omnia/prose/rules/omnia-001.md"),
            &basic_rule("OMNIA-001", "Target overlay", "important"),
        );

        let first = serde_json::to_string_pretty(&resolve_rules(
            rules_root.path(),
            project.path(),
            "omnia",
            &[],
            false,
        ))
        .expect("first pretty");
        let second = serde_json::to_string_pretty(&resolve_rules(
            rules_root.path(),
            project.path(),
            "omnia",
            &[],
            false,
        ))
        .expect("second pretty");
        assert_eq!(first, second, "two consecutive exports must be byte-identical");
    }

    /// CLI smoke test: a hand-built rules-root tree with
    /// a `CORE-*` rule under `codex/rules/core/` excludes that
    /// rule from `specify rules export` by default and includes it under
    /// `--include-core`. Uses `assert_cmd` so the closed CLI plumbing
    /// (clap struct → handler → resolver) is exercised end-to-end.
    #[test]
    fn include_core_flag_toggles_core_rules() {
        let rules_root = tempdir().expect("rules root tempdir");
        let project = tempdir().expect("project tempdir");

        write_rule_fixture(
            &rules_root.path().join("codex/rules/universal/uni-001.md"),
            "UNI-001",
            "Universal anchor",
        );
        // This fixture uses a high out-of-the-way id (`CORE-999`) so the
        // test never collides with a first-party core rule.
        write_rule_fixture(
            &rules_root.path().join("codex/rules/core/CORE-fixture.md"),
            "CORE-999",
            "Core fixture",
        );

        let off = export_via_cli(rules_root.path(), project.path(), false);
        let off_rules =
            off.pointer("/rules").and_then(Value::as_array).expect("rules array on default export");
        assert!(
            !off_rules.iter().any(|r| rule_id(r) == "CORE-999"),
            "CORE-999 must NOT appear without --include-core; got: {off_rules:#?}",
        );
        assert!(
            off_rules.iter().any(|r| rule_id(r) == "UNI-001"),
            "UNI-001 must still appear without --include-core; got: {off_rules:#?}",
        );

        let on = export_via_cli(rules_root.path(), project.path(), true);
        let on_rules = on
            .pointer("/rules")
            .and_then(Value::as_array)
            .expect("rules array with --include-core");
        let core_rule = on_rules
            .iter()
            .find(|r| rule_id(r) == "CORE-999")
            .expect("CORE-999 must appear with --include-core");
        assert_eq!(
            core_rule.pointer("/origin").and_then(Value::as_str),
            Some("core"),
            "core fixture must carry origin=core in the envelope",
        );
    }

    /// Write a minimal rule fixture that satisfies the CH-11 parser and
    /// the codex-rule schema. Mirrors the helper used by the resolver's
    /// unit tests in `crates/standards/src/rules/resolve.rs`.
    fn write_rule_fixture(path: &Path, id: &str, title: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create parent dir");
        }
        let body = format!(
            "---\nid: {id}\ntitle: {title}\nseverity: important\ntrigger: Synthetic fixture trigger sentence long enough for schema.\n---\n\n## Rule\n\nBody for {id}.\n",
        );
        fs::write(path, body).expect("write rule fixture");
    }

    /// Invoke `specify rules export` against an explicit rules root and
    /// parse the JSON envelope on stdout. `include_core` toggles the
    /// closed `--include-core` flag.
    fn export_via_cli(rules_root: &Path, project: &Path, include_core: bool) -> Value {
        let mut cmd = Command::cargo_bin("specify").expect("cargo_bin(specify)");
        cmd.args(["--format", "json", "rules", "export", "--target", "omnia"])
            .args(["--rules-root".as_ref(), rules_root.as_os_str()])
            .args(["--project-dir".as_ref(), project.as_os_str()]);
        if include_core {
            cmd.arg("--include-core");
        }
        let output = cmd.output().expect("specify invocation");
        assert!(
            output.status.success(),
            "specify rules export failed (status: {:?}); stderr:\n{}\nstdout:\n{}",
            output.status,
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout),
        );
        let stdout = std::str::from_utf8(&output.stdout).expect("utf8 stdout");
        serde_json::from_str(stdout)
            .unwrap_or_else(|err| panic!("stdout is not JSON ({err}); raw:\n{stdout}"))
    }

    fn rule_id(rule: &Value) -> &str {
        rule.pointer("/rule-id").and_then(Value::as_str).unwrap_or("")
    }

    /// Negative scenario: the global `--format text` default is rejected
    /// before any resolution work — v1 export emits JSON only, so the
    /// handler returns `Error::Argument` (exit 2) with a hint to rerun
    /// with `--format json`. Exercises the CLI plumbing end-to-end so the
    /// JSON-only contract stays pinned at the wire boundary.
    #[test]
    fn negative_text_format_rejected() {
        let project = tempdir().expect("project tempdir");

        let output = Command::cargo_bin("specify")
            .expect("cargo_bin(specify)")
            .args(["--format", "text", "rules", "export", "--target", "omnia"])
            .args(["--project-dir".as_ref(), project.path().as_os_str()])
            .output()
            .expect("specify invocation");

        assert_eq!(
            output.status.code(),
            Some(2),
            "expected exit 2; stderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("--format json"),
            "stderr must hint the JSON-only contract; got:\n{stderr}"
        );
    }

    /// Negative scenario: a project dir with no shared rules tree, no
    /// `--rules-root`, must exit `2` (validation) with `rules-root-required`
    /// surfaced through the error envelope. Exercises the CH-17 CLI
    /// plumbing end-to-end so the wire contract for the closed
    /// `ResolveError::RulesRootRequired` mapping stays pinned.
    #[test]
    fn negative_rules_root_required() {
        let project = tempdir().expect("project tempdir");

        let output = Command::cargo_bin("specify")
            .expect("cargo_bin(specify)")
            .args(["--format", "json", "rules", "export", "--target", "omnia"])
            .args(["--project-dir".as_ref(), project.path().as_os_str()])
            .output()
            .expect("specify invocation");

        assert_eq!(
            output.status.code(),
            Some(2),
            "expected exit 2; stderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );

        let stderr = std::str::from_utf8(&output.stderr).expect("utf8 stderr");
        let envelope: Value = serde_json::from_str(stderr)
            .unwrap_or_else(|err| panic!("stderr is not JSON ({err}); raw:\n{stderr}"));

        let code = envelope
            .get("error")
            .and_then(Value::as_str)
            .expect("envelope must carry the top-level error code");
        assert_eq!(code, "rules-root-required", "envelope:\n{envelope:#}");
    }
}
