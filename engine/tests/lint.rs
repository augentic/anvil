//! End-to-end binary tests for the `specify lint` surface
//! (`lint framework`, `lint framework --format json`, and `lint
//! project`).

mod support {
    //! Shared fixture scaffold for the `specify lint framework` suites
    //! (`framework.rs`, `framework_json.rs`).

    use std::fs;
    use std::path::Path;

    /// Write the minimal directory and file scaffold that
    /// `Context::from_framework_root` requires *and* that silences every
    /// non-codex authoring check on an otherwise empty tree.
    ///
    /// Specifically the scaffold:
    ///
    /// - Creates `plugins/`, `adapters/{sources,targets,shared}/` so the
    ///   path passes `is_framework_root`.
    /// - Writes a structurally-valid `.cursor-plugin/marketplace.json`
    ///   carrying a single synthetic `test` plugin entry so the
    ///   `plugins.marketplace-drift` schema (`minItems: 1`) is satisfied
    ///   without dragging real plugin content into the tree.
    /// - Writes the matching `plugins/test/.cursor-plugin/plugin.json`
    ///   plus an empty `plugins/test/skills/` directory so the
    ///   `marketplace` framework tool finds the manifest the marketplace
    ///   declares.
    /// - Writes `docs/standards/skill-authoring.md` containing the literal
    ///   `512` (description cap) and `200` (body cap) tokens so
    ///   `prose.numeric-cap-exceeded` short-circuits (the description cap is
    ///   cross-checked against the embedded `skill.schema.json`).
    /// - Writes `docs/reference/review-team-protocol.md` so the
    ///   `agent-teams.missing-canonical` predicate has a canonical doc
    ///   to hash against; per-target `references/agent-teams.md` files
    ///   are never created so the per-adapter overlay arm short-circuits.
    pub fn scaffold_framework(root: &Path) {
        for rel in [
            "adapters/sources",
            "adapters/targets",
            "adapters/shared",
            "plugins",
            "plugins/test/skills",
        ] {
            fs::create_dir_all(root.join(rel)).expect("scaffold dir");
        }

        let marketplace = root.join(".cursor-plugin").join("marketplace.json");
        fs::create_dir_all(marketplace.parent().expect("marketplace parent"))
            .expect("mkdir .cursor-plugin");
        fs::write(
            &marketplace,
            r#"{
  "name": "test",
  "owner": { "name": "Test Owner", "email": "test@example.com" },
  "metadata": {
    "description": "Synthetic marketplace for specify lint framework tests.",
    "version": "0.0.0",
    "pluginRoot": "plugins"
  },
  "plugins": [
    {
      "name": "test",
      "source": "test",
      "description": "Synthetic plugin used by specify lint framework tests."
    }
  ]
}
"#,
        )
        .expect("marketplace.json");

        let plugin_manifest =
            root.join("plugins").join("test").join(".cursor-plugin").join("plugin.json");
        fs::create_dir_all(plugin_manifest.parent().expect("plugin manifest parent"))
            .expect("mkdir plugins/test/.cursor-plugin");
        fs::write(
            &plugin_manifest,
            r#"{
  "name": "test",
  "displayName": "Test Plugin",
  "description": "Synthetic plugin used by specify lint framework tests.",
  "version": "0.0.0"
}
"#,
        )
        .expect("plugins/test/.cursor-plugin/plugin.json");

        let standards = root.join("docs").join("standards").join("skill-authoring.md");
        fs::create_dir_all(standards.parent().expect("standards parent"))
            .expect("mkdir docs/standards");
        fs::write(
        &standards,
        "# Skill authoring (synthetic)\n\nDescription cap: 512 characters. Body cap: 200 lines.\n",
    )
    .expect("skill-authoring.md");

        let canonical = root.join("docs").join("reference").join("review-team-protocol.md");
        fs::create_dir_all(canonical.parent().expect("canonical parent"))
            .expect("mkdir docs/reference");
        fs::write(&canonical, "# Review Team Protocol\n\nSynthetic stub for tests.\n")
            .expect("review-team-protocol.md");
    }
}

mod framework {
    //! End-to-end behavior edges for `specify lint framework` that the JSON
    //! goldens in `framework_json.rs` do not pin:
    //!
    //! - Framework self-lint writes **no** journal: the `lint-completed`
    //!   contract is scoped to `specify lint project` (DECISIONS.md
    //!   §"Journal event names").
    //! - The retired `kind: authoring-predicate` bridge no longer parses.
    //! - A duplicate rule id aborts the run fatally (no degraded skip mode).
    //!
    //! Envelope shape, finding contents, and the human formatter are
    //! covered by the goldens + text smoke in `framework_json.rs`; the
    //! per-kind evaluator semantics live as crate-level unit tests in
    //! `specify-standards`.

    use std::fs;
    use std::path::Path;

    use assert_cmd::Command;
    use serde_json::Value;
    use specify_standards::rules::{HintKind, ParseError, parse_rule};
    use tempfile::TempDir;

    use super::support::scaffold_framework;

    /// Run `specify lint framework --framework-root <root> --output-format json`
    /// and return the captured `(exit, stdout, stderr)` triple.
    fn run_lint_framework(root: &Path, args: &[&str]) -> (Option<i32>, Vec<u8>, Vec<u8>) {
        let output = Command::cargo_bin("specify")
            .expect("cargo_bin(specify)")
            .args(["lint", "framework", "--framework-root"])
            .arg(root)
            .args(args)
            .env("NO_COLOR", "1")
            .output()
            .expect("specify lint framework invocation");
        (output.status.code(), output.stdout, output.stderr)
    }

    /// Framework self-lint writes no journal. The `lint-completed` contract
    /// is scoped to `specify lint project` (DECISIONS.md §"Journal event
    /// names"), so a `specify lint framework` run must not create
    /// `<framework_root>/.specify/journal.jsonl`.
    #[test]
    fn framework_lint_writes_no_journal() {
        let temp = TempDir::new().expect("tempdir");
        scaffold_framework(temp.path());

        let journal_path = temp.path().join(".specify").join("journal.jsonl");
        assert!(!journal_path.exists(), "precondition: journal must not exist before the run");

        let (_code, _stdout, stderr) =
            run_lint_framework(temp.path(), &["--output-format", "json"]);

        assert!(
            !journal_path.exists(),
            "framework self-lint must not journal, but found {}; stderr:\n{}",
            journal_path.display(),
            String::from_utf8_lossy(&stderr),
        );
    }

    /// Write two data rule files that share the id `CORE-100`, which the
    /// `rules.duplicate-rule-id` predicate flags as a whole-tree duplicate.
    /// Both are otherwise schema-valid and carry no hints, so the only
    /// finding the predicate produces is the duplicate-id collision.
    fn write_duplicate_rule_id(root: &Path) {
        let core_dir = root.join("adapters/shared/rules/core");
        fs::create_dir_all(&core_dir).expect("mkdir core rules");
        for file in ["CORE-100-first.md", "CORE-100-second.md"] {
            fs::write(
                core_dir.join(file),
                "---\n\
id: CORE-100\n\
title: Synthetic Duplicate\n\
severity: important\n\
trigger: A synthetic rule used to exercise duplicate-id detection.\n\
---\n\n\
## Rule\n\n\
Synthetic data rule sharing an id with its sibling.\n",
            )
            .expect("write duplicate rule");
        }
    }

    /// Post-bridge invariant: the `kind: authoring-predicate` mechanism is
    /// gone. Rule-agnostic — it pins the
    /// *mechanism*, not any `CORE-NNN`: the closed `HintKind` enum no longer
    /// carries the bridge discriminant, and a rule file that still declares
    /// it fails `rule.schema.json` validation rather than dispatching to an
    /// in-engine imperative predicate. The framework lint therefore resolves
    /// every rule through declarative hints + referenced tools only.
    #[test]
    fn authoring_predicate_kind_is_removed() {
        assert!(
            serde_json::from_value::<HintKind>(Value::String("authoring-predicate".into()))
                .is_err(),
            "HintKind must no longer carry the authoring-predicate bridge variant",
        );

        let rule = "---\n\
id: CORE-999\n\
title: Retired Bridge Kind\n\
severity: important\n\
trigger: A rule that still declares the removed authoring-predicate bridge kind.\n\
rule_hints:\n\
\x20 - kind: authoring-predicate\n\
\x20   value: scenarios.stale-recorded-trace\n\
---\n\n\
## Rule\n\n\
The authoring-predicate bridge has been removed.\n";
        let err = parse_rule(rule).expect_err("the retired bridge kind must no longer parse");
        assert!(
            matches!(err, ParseError::Schema(_)),
            "expected a rule-schema rejection of the retired kind, got: {err:?}",
        );
    }

    /// The engine's resolver-level duplicate-id guard is fatal: when two
    /// rule files share an id the codex fails to resolve and the run aborts
    /// before the declarative pass, surfacing `rules-duplicate-rule-id` on
    /// stderr and exiting 2. There is no degraded "skip the declarative
    /// pass" mode — skipping it would silently pass a broken codex now that
    /// every check (including CORE-026) resolves through declarative hints.
    #[test]
    fn duplicate_rule_id_aborts_fatally() {
        let temp = TempDir::new().expect("tempdir");
        scaffold_framework(temp.path());
        write_duplicate_rule_id(temp.path());

        let (code, stdout, stderr) = run_lint_framework(temp.path(), &["--output-format", "json"]);
        assert_eq!(
            code,
            Some(2),
            "a duplicate rule id must abort the run; stdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&stdout),
            String::from_utf8_lossy(&stderr),
        );
        let stderr_text = String::from_utf8_lossy(&stderr);
        assert!(
            stderr_text.contains("rules-duplicate-rule-id"),
            "the resolver abort must surface the duplicate-id error; stderr:\n{stderr_text}",
        );
    }
}

mod framework_adapters {
    //! End-to-end `specify lint framework` coverage for an adapters-only
    //! framework root (RFC-48 H1): an `adapters/` tree with no `plugins/`
    //! directory and no `.cursor-plugin/marketplace.json`.
    //!
    //! Two invariants this surface owes:
    //!
    //! - The plugin-bound `marketplace` (CORE-022) and `prose` (CORE-024)
    //!   checkers must **no-op** when their inputs are absent, so an
    //!   adapters-only root lints clean.
    //! - The new `extension` checker (CORE-061
    //!   `adapter-extension-crate-missing`) must **fire** when an adapter
    //!   declares `adapter.yaml.extension` without the co-located
    //!   `extension/` crate or the committed `adapter.wasm`.
    //!
    //! The harness mirrors `framework_json.rs`: it drives the binary via
    //! [`assert_cmd::Command::cargo_bin`] against a synthetic tempdir tree
    //! and reads the `--format json` envelope off stdout.

    use std::fs;
    use std::path::Path;

    use assert_cmd::Command;
    use serde_json::Value;
    use tempfile::TempDir;

    /// Run `specify lint framework --framework-root <root> --format json`
    /// and return the captured `(exit, stdout, stderr)` triple.
    fn run_lint_framework_json(root: &Path) -> (Option<i32>, Vec<u8>, Vec<u8>) {
        let output = Command::cargo_bin("specify")
            .expect("cargo_bin(specify)")
            .args(["lint", "framework", "--framework-root"])
            .arg(root)
            .args(["--format", "json"])
            .env("NO_COLOR", "1")
            .output()
            .expect("specify lint framework invocation");
        (output.status.code(), output.stdout, output.stderr)
    }

    /// Write `body` to `<root>/<rel>`, creating any missing parents.
    fn write(root: &Path, rel: &str, body: &str) {
        let path = root.join(rel);
        fs::create_dir_all(path.parent().expect("rel parent")).expect("mkdir rel parent");
        fs::write(path, body).expect("write fixture file");
    }

    /// Scaffold an adapters-only framework root: one source and one target
    /// adapter (each manifest + a brief), with no `plugins/`, no
    /// `.cursor-plugin/marketplace.json`, and no `docs/standards/`.
    fn scaffold_adapters_only(root: &Path) {
        write(
            root,
            "adapters/sources/documentation/adapter.yaml",
            "name: documentation\nversion: \"1.0.0\"\naxis: source\n\
description: Adapters-only source fixture.\n\
briefs:\n  survey: briefs/survey.md\n  extract: briefs/extract.md\n",
        );
        write(
            root,
            "adapters/sources/documentation/briefs/survey.md",
            "# documentation.survey\n\nMinimal brief.\n\n## Inputs\n\n- intent.\n\n## Output contract\n\nLeads.\n",
        );
        write(
            root,
            "adapters/targets/omnia/adapter.yaml",
            "name: omnia\nversion: \"1.0.0\"\naxis: target\n\
description: Adapters-only target fixture.\n\
briefs:\n  shape: briefs/shape.md\n  build: briefs/build.md\n  merge: briefs/merge.md\n",
        );
        write(
            root,
            "adapters/targets/omnia/briefs/shape.md",
            "# omnia.shape\n\nMinimal brief.\n\n## Inputs\n\n- spec.\n\n## Output contract\n\nA reconciled spec.\n",
        );
    }

    /// Synthetic `CORE-022` so the `marketplace` checker runs; with no
    /// `.cursor-plugin/marketplace.json` present it must no-op (H1).
    fn write_marketplace_rule(root: &Path) {
        write(
            root,
            "adapters/shared/rules/core/CORE-022-plugins-marketplace-drift.md",
            "---\nid: CORE-022\ntitle: Plugins Marketplace Drift\nseverity: important\n\
trigger: marketplace.json drifts from on-disk plugin layout.\n\
rule_hints:\n  - kind: path-pattern\n    value: adapters/shared/rules/core/CORE-022-plugins-marketplace-drift.md\n  - kind: tool\n    value: marketplace\n---\n\n\
## Rule\n\nSynthetic CORE-022 for adapters-only tests.\n",
        );
    }

    /// Synthetic `CORE-024` so the `prose` checker runs; with no
    /// `docs/standards/skill-authoring.md` present it must no-op (H1).
    fn write_prose_rule(root: &Path) {
        write(
            root,
            "adapters/shared/rules/core/CORE-024-prose-numeric-cap-exceeded.md",
            "---\nid: CORE-024\ntitle: Prose Numeric Cap Exceeded\nseverity: important\n\
trigger: A documented skill numeric cap drifted from its canonical source.\n\
rule_hints:\n  - kind: path-pattern\n    value: adapters/shared/rules/core/CORE-024-prose-numeric-cap-exceeded.md\n  - kind: tool\n    value: prose\n    config:\n      description-cap: 512\n      body-cap: 200\n---\n\n\
## Rule\n\nSynthetic CORE-024 for adapters-only tests.\n",
        );
    }

    /// Synthetic `CORE-061` so the `extension` checker runs.
    fn write_extension_rule(root: &Path) {
        write(
            root,
            "adapters/shared/rules/core/CORE-061-adapter-extension-crate-missing.md",
            "---\nid: CORE-061\ntitle: Adapter Extension Crate Missing\nseverity: important\n\
trigger: adapter.yaml declares an extension block but the co-located crate or committed adapter.wasm is missing.\n\
rule_hints:\n  - kind: path-pattern\n    value: adapters/shared/rules/core/CORE-061-adapter-extension-crate-missing.md\n  - kind: tool\n    value: extension\n---\n\n\
## Rule\n\nSynthetic CORE-061 for adapters-only tests.\n",
        );
    }

    /// An adapters-only root with the plugin-bound `marketplace` and `prose`
    /// rules present but their inputs (`marketplace.json`,
    /// `skill-authoring.md`) absent lints clean: both checkers no-op (H1).
    #[test]
    fn adapters_only_root_lints_clean() {
        let temp = TempDir::new().expect("tempdir");
        scaffold_adapters_only(temp.path());
        write_marketplace_rule(temp.path());
        write_prose_rule(temp.path());

        let (code, stdout, stderr) = run_lint_framework_json(temp.path());
        assert_eq!(
            code,
            Some(0),
            "adapters-only root must lint clean; stderr:\n{}\nstdout:\n{}",
            String::from_utf8_lossy(&stderr),
            String::from_utf8_lossy(&stdout),
        );

        let envelope: Value = serde_json::from_slice(&stdout).expect("stdout is JSON");
        let findings = envelope.get("findings").and_then(Value::as_array).expect("findings array");
        assert!(
            findings.is_empty(),
            "marketplace + prose checkers must no-op on absent inputs; got:\n{}",
            String::from_utf8_lossy(&stdout),
        );
    }

    /// CORE-061 fires when an adapter declares `adapter.yaml.extension` but
    /// ships neither the co-located `extension/` crate nor a committed
    /// `adapter.wasm`.
    #[test]
    fn extension_rule_fires_for_missing_crate() {
        let temp = TempDir::new().expect("tempdir");
        scaffold_adapters_only(temp.path());
        write_extension_rule(temp.path());
        write(
            temp.path(),
            "adapters/targets/withext/adapter.yaml",
            "name: withext\nversion: \"1.0.0\"\naxis: target\n\
description: Declares an extension without the co-located crate.\n\
briefs:\n  shape: briefs/shape.md\n  build: briefs/build.md\n  merge: briefs/merge.md\n\
extension:\n  name: withext\n  permissions:\n    read:\n      - $PROJECT_DIR\n",
        );
        write(
            temp.path(),
            "adapters/targets/withext/briefs/shape.md",
            "# withext.shape\n\nMinimal brief.\n",
        );

        let (code, stdout, stderr) = run_lint_framework_json(temp.path());
        assert_eq!(
            code,
            Some(2),
            "a declared extension with no crate/wasm must block; stderr:\n{}\nstdout:\n{}",
            String::from_utf8_lossy(&stderr),
            String::from_utf8_lossy(&stdout),
        );

        let envelope: Value = serde_json::from_slice(&stdout).expect("stdout is JSON");
        let findings = envelope.get("findings").and_then(Value::as_array).expect("findings array");
        assert!(
            findings.iter().any(|f| f.get("rule-id").and_then(Value::as_str) == Some("CORE-061")),
            "expected a CORE-061 extension finding; got:\n{}",
            String::from_utf8_lossy(&stdout),
        );
    }
}

mod framework_json {
    //! Golden tests for `specify lint framework --format json`.
    //!
    //! These tests pin the byte-stable wire envelope emitted by the
    //! `specify lint framework` `--format json` mode (CH-22 plumbing, CH-21
    //! finding mapper, CH-20 severity table) against representative
    //! synthetic framework trees. They exercise the binary surface
    //! directly via [`assert_cmd::Command::cargo_bin("specify")`] so the
    //! full CLI plumbing — argument parsing, dispatch, envelope emit,
    //! exit-code mapping — stays under test the way RM-10 / CI
    //! integrations will consume it.
    //!
    //! ## Path-normalisation strategy
    //!
    //! `Context::from_framework_root` canonicalises the supplied path,
    //! so every finding's `location.path` carries the absolute,
    //! canonicalised location of the file inside the test's
    //! `tempfile::TempDir`. That path is machine-specific (e.g.
    //! `/private/var/folders/.tmpXXXXXX/...` on macOS,
    //! `/tmp/.tmpXXXXXX/...` on Linux) and would make any golden file
    //! non-portable. Worse, the structured lint fingerprint algorithm hashes the
    //! raw path, so a naive prefix-swap on the wire JSON would carry
    //! stale fingerprints that no consumer could re-verify.
    //!
    //! We instead normalise inside the test, before golden comparison:
    //!
    //! 1. Capture the binary's pretty-printed JSON envelope from stdout.
    //! 2. For each finding, deserialise into the typed
    //!    [`Diagnostic`], swap any `location.path` prefix that
    //!    matches the canonicalised tempdir root with the literal
    //!    `<FRAMEWORK_ROOT>` placeholder.
    //! 3. Recompute the fingerprint via
    //!    [`specify_diagnostics::fingerprint`] against
    //!    the normalised finding. The stored fingerprint then reflects
    //!    the placeholder-anchored canonical preimage.
    //! 4. Re-serialise and compare/regenerate the placeholder-anchored
    //!    envelope against `tests/fixtures/lint-framework/<name>.json`.
    //!
    //! The resulting goldens are machine-portable and self-consistent:
    //! consumers replaying the test on any host produce the same path
    //! strings *and* the same fingerprints. This deliberately keeps the
    //! mapper (CH-21) and finding fingerprint algorithm (CH-15)
    //! untouched — the normalisation lives in the test harness only.
    //!
    //! ## Regenerating goldens
    //!
    //! After an intentional change to the envelope shape, mapper, or
    //! check predicates:
    //!
    //! ```text
    //! REGENERATE_GOLDENS=1 cargo nextest run --test lint
    //! ```
    //!
    //! The helper writes goldens as `serde_json::to_string_pretty` +
    //! trailing newline, matching the CH-18 `tests/codex_export.rs`
    //! pattern.

    use std::path::{Path, PathBuf};
    use std::{env, fs};

    use assert_cmd::Command;
    use serde_json::{Value, json};
    use specify_diagnostics::{Diagnostic, fingerprint, validate_diagnostic_json};
    use tempfile::TempDir;

    use super::support::scaffold_framework;

    /// Replacement token for the canonicalised framework-root prefix in
    /// every captured `location.path`. Chosen so it cannot occur in a
    /// real filesystem path.
    const FRAMEWORK_ROOT_PLACEHOLDER: &str = "<FRAMEWORK_ROOT>";

    /// Resolve the directory where golden fixtures live.
    fn goldens_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("lint-framework")
    }

    /// The shared `super::support::scaffold_framework` base tree plus the
    /// declarative `CORE-001` rule the golden suite needs (see
    /// [`write_core_adapter_schema_rule`]).
    fn write_scaffold(root: &Path) {
        scaffold_framework(root);
        write_core_adapter_schema_rule(root);
    }

    /// Declarative `CORE-001` so adapter manifest schema violations surface
    /// without relying on rule-file parse failures (which abort codex resolve).
    fn write_core_adapter_schema_rule(root: &Path) {
        fs::create_dir_all(root.join("adapters/shared/rules/core")).expect("core rules dir");
        write_codex_rule(
            root,
            "adapters/shared/rules/core/CORE-001-adapter-schema.md",
            r"---
id: CORE-001
title: Adapter Manifest Schema
severity: critical
trigger: An adapter manifest fails adapter.schema.json validation.
rule_hints:
  - kind: path-pattern
    value: adapters/**/adapter.yaml
  - kind: schema
    value: adapter
---

## Rule

Synthetic CORE-001 for golden tests.

## Look For

Invalid manifests.

## Fix

Fix manifest.
",
        );
    }

    /// Declarative `CORE-009` driving the `rules` WASI tool's namespace
    /// ownership check. The owner→prefix map, source-axis prefixes, and
    /// reserved-namespace owners travel in `config:`, mirroring the live rule
    /// in `augentic/specify`. A sentinel `path-pattern` runs the whole-tree
    /// tool exactly once.
    fn write_core_namespace_owner_rule(root: &Path) {
        fs::create_dir_all(root.join("adapters/shared/rules/core")).expect("core rules dir");
        write_codex_rule(
            root,
            "adapters/shared/rules/core/CORE-009-rule-namespace-owner.md",
            r"---
id: CORE-009
title: Rule Namespace Owner
severity: important
trigger: A rule id namespace prefix is not owned by its rules directory.
rule_hints:
  - kind: path-pattern
    value: adapters/shared/rules/core/CORE-009-rule-namespace-owner.md
  - kind: tool
    value: rules
    config:
      owner-prefixes:
        universal: [UNI]
        core: [CORE]
        omnia: [OMNIA, RUST, SEC]
        contracts: [IFACE]
        vectis: [VECTIS]
      source-axis-prefixes: [SRC]
      reserved-namespaces:
        FRAME: universal
---

## Rule

Synthetic CORE-009 for golden tests; the owner policy lives in config.

## Look For

Misplaced namespace prefixes.

## Fix

Move the rule to the directory that owns its prefix.
",
        );
    }

    /// Write a minimal source-adapter manifest at
    /// `adapters/sources/<name>/adapter.yaml` so
    /// `adapter.missing-manifest` does not fire when a `<name>` source
    /// adapter directory is created (e.g. by writing a rule under
    /// `adapters/sources/<name>/rules/`).
    fn write_source_adapter_manifest(root: &Path, name: &str) {
        let path = root.join("adapters").join("sources").join(name).join("adapter.yaml");
        fs::create_dir_all(path.parent().expect("source adapter parent"))
            .expect("mkdir source adapter parent");
        fs::write(
            &path,
            format!(
                r"name: {name}
version: 1.0.0
axis: source
description: Synthetic source adapter for specify lint framework golden tests.
briefs:
  survey: briefs/survey.md
  extract: briefs/extract.md
"
            ),
        )
        .expect("source adapter.yaml");
    }

    /// Render a structurally-valid rule body with the supplied id.
    fn valid_rule_body(id: &str) -> String {
        format!(
            r"---
id: {id}
title: Synthetic Test Rule
severity: important
trigger: When the test harness needs a structurally-valid rule.
---

## Rule

Body preserved so the rule passes shape validation.
"
        )
    }

    /// Write a rule file under `<root>/<rel_path>`, creating any
    /// missing parents.
    fn write_codex_rule(root: &Path, rel_path: &str, body: &str) {
        let path = root.join(rel_path);
        fs::create_dir_all(path.parent().expect("rule parent")).expect("mkdir rule parent");
        fs::write(&path, body).expect("write rule");
    }

    /// Write a minimal target-adapter manifest at
    /// `adapters/targets/<name>/adapter.yaml` that validates against
    /// `target.schema.json`. The brief paths are strings only — they
    /// never need to resolve on disk for the schema or brief-size
    /// predicates to short-circuit.
    fn write_target_adapter_manifest(root: &Path, name: &str) {
        let path = root.join("adapters").join("targets").join(name).join("adapter.yaml");
        fs::create_dir_all(path.parent().expect("adapter parent")).expect("mkdir adapter parent");
        fs::write(
            &path,
            format!(
                r"name: {name}
version: 1.0.0
axis: target
description: Synthetic target adapter for specify lint framework golden tests.
briefs:
  shape: briefs/shape.md
  build: briefs/build.md
  merge: briefs/merge.md
"
            ),
        )
        .expect("adapter.yaml");
    }

    /// Run `specify lint framework --framework-root <root> --format json` and
    /// return the (exit code, stdout, stderr) triple.
    fn run_lint_framework_json(root: &Path) -> (Option<i32>, Vec<u8>, Vec<u8>) {
        let output = Command::cargo_bin("specify")
            .expect("cargo_bin(specify)")
            .args(["lint", "framework", "--framework-root"])
            .arg(root)
            .args(["--format", "json"])
            .output()
            .expect("specify lint framework invocation");
        (output.status.code(), output.stdout, output.stderr)
    }

    /// Canonicalise `framework_root` exactly the way `Context` does, so
    /// the prefix we strip from `location.path` matches the absolute
    /// path the binary emitted on stdout.
    fn canonical_prefix(framework_root: &Path) -> String {
        framework_root
            .canonicalize()
            .expect("canonicalize framework_root")
            .to_string_lossy()
            .replace('\\', "/")
    }

    /// Replace the canonical-tempdir prefix on every finding's
    /// `location.path` with the [`FRAMEWORK_ROOT_PLACEHOLDER`] sentinel,
    /// then recompute the fingerprint so the stored hash matches the
    /// placeholder-anchored preimage. Returns the rewritten envelope.
    ///
    /// Findings without a `location` field or whose `path` does not
    /// start with the canonical tempdir are passed through untouched
    /// (still re-fingerprinted to stay self-consistent if some other
    /// field happened to change — defensive, currently a no-op).
    fn normalize_envelope(envelope: Value, framework_root: &Path) -> Value {
        let prefix = canonical_prefix(framework_root);
        let mut envelope = envelope;

        let Some(findings) = envelope.get_mut("findings").and_then(Value::as_array_mut) else {
            return envelope;
        };

        for finding_json in findings.iter_mut() {
            let mut finding: Diagnostic = serde_json::from_value(finding_json.clone())
                .expect("finding must deserialise into Diagnostic");
            if let Some(location) = finding.location.as_mut() {
                let raw = location.path.replace('\\', "/");
                if let Some(rest) = raw.strip_prefix(&prefix) {
                    location.path = format!("{FRAMEWORK_ROOT_PLACEHOLDER}{rest}");
                } else {
                    location.path = raw;
                }
            }
            finding.fingerprint = fingerprint(&finding);
            *finding_json = serde_json::to_value(&finding).expect("finding must reserialise");
        }

        envelope
    }

    /// Compare `actual` against `<goldens_dir>/<name>.json`, or write
    /// the fixture when `REGENERATE_GOLDENS` is set. Mirrors the CH-18
    /// `tests/codex_export.rs` helper byte-for-byte (pretty-printed
    /// JSON, single trailing newline).
    #[track_caller]
    fn assert_golden(actual: &Value, name: &str) {
        let golden_path = goldens_dir().join(format!("{name}.json"));
        let mut rendered = serde_json::to_string_pretty(actual).expect("pretty json");
        rendered.push('\n');

        if env::var_os("REGENERATE_GOLDENS").is_some() {
            fs::create_dir_all(golden_path.parent().expect("golden parent"))
                .expect("mkdir golden parent");
            fs::write(&golden_path, &rendered).expect("write golden");
            return;
        }

        let expected = fs::read_to_string(&golden_path).unwrap_or_else(|err| {
            panic!(
                "golden {} missing ({err}); regenerate via \
             REGENERATE_GOLDENS=1 cargo nextest run --test lint",
                golden_path.display()
            )
        });

        assert_eq!(
            rendered,
            expected,
            "golden divergence at {}\n--- actual (truncated head) ---\n{}\n--- expected (truncated head) ---\n{}",
            golden_path.display(),
            rendered.chars().take(400).collect::<String>(),
            expected.chars().take(400).collect::<String>(),
        );
    }

    /// (1) A clean framework tree — one valid `SRC-001` rule plus the
    /// scaffold prerequisites — emits the all-zero envelope.
    #[test]
    fn clean_tree_emits_empty_envelope() {
        let temp = TempDir::new().expect("tempdir");
        write_scaffold(temp.path());
        write_source_adapter_manifest(temp.path(), "documentation");
        write_codex_rule(
            temp.path(),
            "adapters/sources/documentation/rules/src-001.md",
            &valid_rule_body("SRC-001"),
        );

        let (code, stdout, stderr) = run_lint_framework_json(temp.path());
        assert_eq!(
            code,
            Some(0),
            "expected exit 0 for clean tree; stderr:\n{}",
            String::from_utf8_lossy(&stderr),
        );

        let envelope: Value = serde_json::from_slice(&stdout).expect("stdout is JSON");
        assert_eq!(
            envelope,
            json!({
                "version": 1,
                "summary": {
                    "critical": 0,
                    "important": 0,
                    "suggestion": 0,
                    "optional": 0,
                },
                "findings": [],
            }),
        );
    }

    /// (2) A framework tree carrying one schema violation, one
    /// namespace-ownership violation, and one duplicate-id violation
    /// emits the populated envelope captured by
    /// `tests/fixtures/lint-framework/violations.json`. Every finding in
    /// the envelope is additionally schema-validated via
    /// [`validate_diagnostic_json`] (CH-16) — covering scenario (3) from
    /// CH-23 in the same test pass.
    #[test]
    fn violations_tree_emits_expected_envelope() {
        let temp = TempDir::new().expect("tempdir");
        write_scaffold(temp.path());

        write_codex_rule(
            temp.path(),
            "adapters/shared/rules/universal/uni-999.md",
            &valid_rule_body("UNI-999"),
        );
        write_core_namespace_owner_rule(temp.path());
        write_target_adapter_manifest(temp.path(), "omnia");
        let bad_manifest = temp.path().join("adapters/targets/omnia/adapter.yaml");
        fs::write(&bad_manifest, "name: omnia\nversion: 1.0.0\naxis: target\n")
            .expect("bad manifest");
        write_codex_rule(
            temp.path(),
            "adapters/targets/omnia/rules/frame-misplaced.md",
            &valid_rule_body("FRAME-001"),
        );

        let (code, stdout, stderr) = run_lint_framework_json(temp.path());
        assert_eq!(
            code,
            Some(2),
            "expected exit 2 for findings; stderr:\n{}",
            String::from_utf8_lossy(&stderr),
        );

        let envelope: Value = serde_json::from_slice(&stdout).expect("stdout is JSON");
        let normalized = normalize_envelope(envelope, temp.path());

        let findings = normalized
            .get("findings")
            .and_then(Value::as_array)
            .expect("normalized envelope carries findings array");
        assert!(
            findings.len() >= 2,
            "expected at least two findings (CORE-001 adapter schema, CORE-009 namespace); got {}",
            findings.len(),
        );
        for finding_json in findings {
            validate_diagnostic_json(finding_json)
                .expect("every finding must validate against the review/finding.schema.json");
        }

        assert_golden(&normalized, "violations");
    }

    /// (4) `--format json` against a non-existent framework root surfaces
    /// the infrastructure error as exit code 1 and still emits a valid
    /// (empty-findings) envelope on stdout. The failure now routes through
    /// the shared runtime `output::report` (A19), so `--format json`
    /// renders the structured `ErrorBody` envelope on stderr — carrying
    /// the `framework-root` discriminant — exactly as `specify
    /// --format json` does, rather than a bespoke `error:` text line.
    #[test]
    fn missing_framework_root_emits_envelope() {
        let temp = TempDir::new().expect("tempdir");
        let missing = temp.path().join("does-not-exist");

        let output = Command::cargo_bin("specify")
            .expect("cargo_bin(specify)")
            .args(["lint", "framework", "--framework-root"])
            .arg(&missing)
            .args(["--format", "json"])
            .output()
            .expect("specify lint framework invocation");

        assert_eq!(
            output.status.code(),
            Some(1),
            "expected exit 1 for infrastructure error; stderr:\n{}",
            String::from_utf8_lossy(&output.stderr),
        );

        let envelope: Value = serde_json::from_slice(&output.stdout)
            .expect("stdout must remain a JSON envelope even on infra error");
        assert_eq!(
            envelope,
            json!({
                "version": 1,
                "summary": {
                    "critical": 0,
                    "important": 0,
                    "suggestion": 0,
                    "optional": 0,
                },
                "findings": [],
            }),
        );

        let stderr = String::from_utf8(output.stderr).expect("utf8 stderr");
        let error_body: Value = serde_json::from_str(&stderr).unwrap_or_else(|err| {
            panic!("stderr must be the JSON ErrorBody envelope ({err}); got:\n{stderr}")
        });
        assert_eq!(
            error_body.get("error").and_then(Value::as_str),
            Some("framework-root"),
            "stderr envelope must carry the infrastructure-error discriminant; got:\n{stderr}",
        );
        assert_eq!(
            error_body.get("exit-code").and_then(Value::as_u64),
            Some(1),
            "stderr envelope must report exit-code 1; got:\n{stderr}",
        );
    }

    /// (5) Default text output on a clean tree now prints the
    /// diagnostics-formatter set's pretty summary line from the
    /// `specify lint framework` extension. Specifically: a `0 finding(s)`
    /// header and a `Summary: 0 critical, 0 important, ...` tally,
    /// driven by `specify_diagnostics::render` with `Format::Pretty`.
    #[test]
    fn text_output_renders_summary() {
        let temp = TempDir::new().expect("tempdir");
        write_scaffold(temp.path());
        write_source_adapter_manifest(temp.path(), "documentation");
        write_codex_rule(
            temp.path(),
            "adapters/sources/documentation/rules/src-001.md",
            &valid_rule_body("SRC-001"),
        );

        let output = Command::cargo_bin("specify")
            .expect("cargo_bin(specify)")
            .args(["lint", "framework", "--framework-root"])
            .arg(temp.path())
            .env("NO_COLOR", "1")
            .output()
            .expect("specify lint framework invocation");

        assert_eq!(
            output.status.code(),
            Some(0),
            "expected exit 0; stderr:\n{}",
            String::from_utf8_lossy(&output.stderr),
        );

        let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
        assert!(
            stdout.contains("0 finding(s)") && stdout.contains("Summary: 0 critical"),
            "expected pretty diagnostics summary on stdout; got:\n{stdout}",
        );
    }
}

mod project {
    //! End-to-end binary tests for `specify lint project`.
    //!
    //! Exercises the wired clap surface, `--rules-root` resolution,
    //! the `--dump-model` debug branch, and the lint exit-code map for the
    //! `rules-root-required` negative scenario. The deterministic happy
    //! path uses a single shared `kind: regex` UNI-100 rule that matches a
    //! literal `TODO` token in the project — chosen because the regex
    //! evaluator is the simplest hint that surfaces an
    //! `important` finding without requiring a WASI tool to be built.
    //!
    //! The per-kind hint behaviour (`path-pattern`, `presence`,
    //! `field-grammar`, `set-coverage`, `cardinality`, `reference-resolves`,
    //! `fenced-block`) is owned at the crate level by the eval `mod unit`
    //! suites in `crates/standards/src/lint/eval/*` and the
    //! `crates/standards/tests/lint_hint/` integration cases. This binary
    //! surface keeps only the wiring smoke (`review_emits_important_exits_2`,
    //! the regex happy path) and the severity-gating exit decision
    //! (`suggestion_finding_present_exits_0`, `blocking_tier_drives_exit`).

    use std::fs;
    use std::path::Path;

    use assert_cmd::Command;
    use jsonschema::{Registry, Resource, Validator};
    use serde_json::Value;
    use specify_schema::{
        DIAGNOSTIC_JSON_SCHEMA, DIAGNOSTIC_REPORT_JSON_SCHEMA, WORKSPACE_MODEL_JSON_SCHEMA,
    };
    use tempfile::TempDir;

    const FINDING_SCHEMA_URL: &str =
        "https://github.com/augentic/specify/schemas/diagnostics/diagnostic.schema.json";

    /// Compile the diagnostic-report envelope schema with the
    /// `diagnostic.schema.json` child resource wired through a
    /// `jsonschema::Registry`. Mirrors the
    /// `specify_diagnostics::render` with `Format::Json` setup so the
    /// e2e test re-validates the same shape the CLI emits.
    fn compile_review_result_validator() -> Validator {
        let envelope: Value =
            serde_json::from_str(DIAGNOSTIC_REPORT_JSON_SCHEMA).expect("envelope schema");
        let finding: Value = serde_json::from_str(DIAGNOSTIC_JSON_SCHEMA).expect("finding schema");
        let registry = Registry::new()
            .add(FINDING_SCHEMA_URL, Resource::from_contents(finding))
            .and_then(jsonschema::RegistryBuilder::prepare)
            .expect("registry build");
        jsonschema::options().with_registry(&registry).build(&envelope).expect("validator build")
    }

    fn compile_workspace_model_validator() -> Validator {
        let schema: Value =
            serde_json::from_str(WORKSPACE_MODEL_JSON_SCHEMA).expect("parse schema");
        jsonschema::validator_for(&schema).expect("validator build")
    }

    #[track_caller]
    fn assert_validates(validator: &Validator, stdout: &str, schema_label: &str) {
        let instance: Value = serde_json::from_str(stdout)
            .unwrap_or_else(|err| panic!("stdout is not JSON ({err}); raw:\n{stdout}"));
        let errors: Vec<String> =
            validator.iter_errors(&instance).map(|err| err.to_string()).collect();
        assert!(
            errors.is_empty(),
            "stdout failed schema validation ({schema_label}): {errors:?}; raw:\n{stdout}"
        );
    }

    /// Scratch workspace used by the happy-path scenarios.
    ///
    /// Lives in the tempdir until the test returns. Two trees are produced:
    ///
    /// - `project_dir/` — a minimal initialised project (`.specify/project.yaml`
    ///   declaring the `contract` tool so the `kind: tool` hint family at
    ///   least passes the `kind: tool` evaluator contract `is_declared` half) plus a `notes.md` file
    ///   carrying the literal `TODO` token the UNI-100 regex hint matches.
    /// - `codex_dir/` — a fresh rules tree with one shared rule under
    ///   `adapters/shared/rules/universal/uni-100.md`. The rule's
    ///   `kind: regex` hint pattern is `TODO`.
    struct Fixture {
        _root: TempDir,
        project: std::path::PathBuf,
        codex: std::path::PathBuf,
    }

    fn build_fixture() -> Fixture {
        let root = TempDir::new().expect("create tempdir");
        let project = root.path().join("project");
        let codex = root.path().join("rules");
        fs::create_dir_all(project.join(".specify")).expect("mkdir project/.specify");
        fs::create_dir_all(codex.join("adapters/shared/rules/universal")).expect("mkdir codex");

        fs::write(
            project.join(".specify").join("project.yaml"),
            concat!(
                "name: review-e2e\n",
                "tools:\n",
                "  - name: contract\n",
                "    version: 0.1.0\n",
                "    source: https://example.com/contract.wasm\n",
            ),
        )
        .expect("write project.yaml");

        fs::write(project.join("notes.md"), "# Project notes\n\nTODO: drop scaffolding.\n")
            .expect("write notes.md");

        fs::write(
            codex.join("adapters/shared/rules/universal/uni-100.md"),
            concat!(
                "---\n",
                "id: UNI-100\n",
                "title: Forbid scaffolding TODOs\n",
                "severity: important\n",
                "trigger: TODO comments leak development scaffolding into shipped artefacts.\n",
                "lint_mode: deterministic\n",
                "rule_hints:\n",
                "  - kind: regex\n",
                "    value: TODO\n",
                "---\n",
                "## Rule\n",
                "\n",
                "Strip scaffolding TODOs before merge.\n",
            ),
        )
        .expect("write UNI-100");

        Fixture {
            _root: root,
            project,
            codex,
        }
    }

    /// Write a `kind: regex` UNI rule into `codex` at the given severity.
    ///
    /// Mirrors the inline rule `build_fixture` writes, but parameterised on
    /// `id` / `severity` / `pattern` so the blocking-tier tests can stand up
    /// `suggestion`-severity rules (which never gate) alongside the default
    /// `important` one (which does).
    fn write_regex_rule(codex: &Path, id: &str, severity: &str, pattern: &str) {
        let slug = id.to_ascii_lowercase();
        fs::write(
            codex.join(format!("adapters/shared/rules/universal/{slug}.md")),
            format!(
                "---\n\
             id: {id}\n\
             title: Forbid scaffolding {pattern}\n\
             severity: {severity}\n\
             trigger: {pattern} tokens leak development scaffolding into shipped artefacts.\n\
             lint_mode: deterministic\n\
             rule_hints:\n\
             \x20 - kind: regex\n\
             \x20   value: {pattern}\n\
             ---\n\
             ## Rule\n\nStrip scaffolding {pattern} before merge.\n",
            ),
        )
        .unwrap_or_else(|err| panic!("write rule {id}: {err}"));
    }

    fn run_review(project: &Path, codex: Option<&Path>, extra: &[&str]) -> std::process::Output {
        let mut cmd = Command::cargo_bin("specify").expect("cargo_bin(specify)");
        // The global `--format` toggles the error-envelope shape; the
        // per-subcommand `--output-format` selects the the diagnostics formatter set closed set.
        cmd.arg("--format").arg("json");
        cmd.arg("lint").arg("project");
        cmd.arg("--target").arg("omnia");
        cmd.arg("--project-dir").arg(project);
        cmd.arg("--output-format").arg("json");
        if let Some(codex) = codex {
            cmd.arg("--rules-root").arg(codex);
        }
        cmd.env_remove("RULES_ROOT");
        for arg in extra {
            cmd.arg(arg);
        }
        cmd.output().expect("specify invocation")
    }

    /// Happy path: a single `important` finding from the UNI-100 regex
    /// hint lands stdout on a schema-valid review envelope and exits 2 per
    /// lint exit mapping.
    #[test]
    fn review_emits_important_exits_2() {
        let fx = build_fixture();
        let output = run_review(&fx.project, Some(&fx.codex), &[]);

        assert_eq!(
            output.status.code(),
            Some(2),
            "expected exit 2; stderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = std::str::from_utf8(&output.stdout).expect("utf8 stdout");
        let validator = compile_review_result_validator();
        assert_validates(&validator, stdout, "review-result");
        let envelope: Value = serde_json::from_str(stdout).expect("parse envelope");
        let important = envelope
            .pointer("/summary/important")
            .and_then(Value::as_u64)
            .expect("summary.important present");
        assert!(
            important >= 1,
            "expected ≥1 important finding, got {important}; envelope:\n{envelope:#}"
        );

        let rule_id = envelope
            .pointer("/findings/0/rule-id")
            .and_then(Value::as_str)
            .expect("findings[0].rule-id");
        assert_eq!(rule_id, "UNI-100", "envelope:\n{envelope:#}");
    }

    /// Bare `specify lint` without a subcommand must fail at clap parse time.
    #[test]
    fn bare_lint_requires_subcommand() {
        let mut cmd = Command::cargo_bin("specify").expect("cargo_bin(specify)");
        cmd.arg("lint");
        let output = cmd.output().expect("specify invocation");

        assert!(
            !output.status.success(),
            "bare `specify lint` must fail; stdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stderr),
            String::from_utf8_lossy(&output.stdout)
        );
        assert!(
            combined.contains("project") || combined.contains("subcommand"),
            "failure must hint at required subcommand; got:\n{combined}"
        );
    }

    /// `DiagnosticReport` envelope byte-stability: two back-to-back runs against the same fixture
    /// must emit byte-identical stdout. Pins the deterministic ordering
    /// contract through the CLI boundary.
    #[test]
    fn review_run_byte_stable() {
        let fx = build_fixture();
        let first = run_review(&fx.project, Some(&fx.codex), &[]);
        let second = run_review(&fx.project, Some(&fx.codex), &[]);
        assert_eq!(
            first.stdout, second.stdout,
            "consecutive specify lint runs must emit byte-identical stdout"
        );
    }

    /// `--dump-model` skips evaluation, emits a `WorkspaceModel` envelope
    /// that validates against the workspace-model schema, and exits 0.
    #[test]
    fn review_dump_model_exits_0() {
        let fx = build_fixture();
        let output = run_review(&fx.project, Some(&fx.codex), &["--dump-model"]);

        assert_eq!(
            output.status.code(),
            Some(0),
            "expected exit 0; stderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = std::str::from_utf8(&output.stdout).expect("utf8 stdout");
        let validator = compile_workspace_model_validator();
        assert_validates(&validator, stdout, "workspace-model");
    }

    /// Journal event: every completed scan appends one
    /// `lint-completed` line to `.specify/journal.jsonl` with the closed
    /// `snake_case` payload shape. The fixture wires a Markdown directive
    /// that demotes the UNI-100 TODO finding so the asserted counts
    /// straddle both buckets (`ignored: 1`, `open: 0`) and the scan exits
    /// clean (`exit_code: 0`) — proving the journal `exit_code` mirrors
    /// the status-aware exit decision the exit and presentation
    /// semantics define.
    #[test]
    fn emits_lint_completed_event() {
        use std::path::PathBuf;

        let root = TempDir::new().expect("create tempdir");
        let project: PathBuf = root.path().join("project");
        let codex: PathBuf = root.path().join("rules");
        fs::create_dir_all(project.join(".specify")).expect("mkdir project/.specify");
        fs::create_dir_all(codex.join("adapters/shared/rules/universal")).expect("mkdir codex");

        fs::write(project.join(".specify").join("project.yaml"), "name: review-journal-e2e\n")
            .expect("write project.yaml");

        // `<!-- specify-ignore: UNI-100 — … -->` lands on line 2 so the
        // directive's `target_line` resolves to the next non-blank,
        // non-comment line: the TODO on line 3.
        fs::write(
            project.join("notes.md"),
            concat!(
                "# Project notes\n",
                "<!-- specify-ignore: UNI-100 — accepted tech-debt sentinel for the demo -->\n",
                "TODO: drop scaffolding.\n",
            ),
        )
        .expect("write notes.md");

        fs::write(
            codex.join("adapters/shared/rules/universal/uni-100.md"),
            concat!(
                "---\n",
                "id: UNI-100\n",
                "title: Forbid scaffolding TODOs\n",
                "severity: important\n",
                "trigger: TODO comments leak development scaffolding into shipped artefacts.\n",
                "lint_mode: deterministic\n",
                "rule_hints:\n",
                "  - kind: regex\n",
                "    value: TODO\n",
                "---\n",
                "## Rule\n\nStrip scaffolding TODOs before merge.\n",
            ),
        )
        .expect("write UNI-100");

        let output = run_review(&project, Some(&codex), &[]);
        assert_eq!(
            output.status.code(),
            Some(0),
            "directive demotes the only finding to `ignored`; scan must exit 0; stderr:\n{}",
            String::from_utf8_lossy(&output.stderr),
        );

        let raw = fs::read_to_string(project.join(".specify").join("journal.jsonl"))
            .expect("read journal.jsonl");
        let last_line =
            raw.lines().rfind(|l| !l.is_empty()).expect("journal must contain at least one line");
        let event: Value = serde_json::from_str(last_line)
            .unwrap_or_else(|err| panic!("last journal line is not JSON ({err}): {last_line}"));

        assert_eq!(
            event.pointer("/event").and_then(Value::as_str),
            Some("lint-completed"),
            "last journal line must be the lint-completed event; got:\n{event:#}",
        );

        let payload = event.get("payload").expect("payload object present");
        assert_eq!(payload.pointer("/scope/target").and_then(Value::as_str), Some("omnia"));
        assert!(
            payload.pointer("/scope/slice").is_some_and(Value::is_null),
            "slice must serialise to JSON null when --slice is absent; payload:\n{payload:#}",
        );
        assert!(
            payload.pointer("/scope/artifact").is_some_and(Value::is_null),
            "artifact must serialise to JSON null on full scans; payload:\n{payload:#}",
        );
        assert_eq!(
            payload.pointer("/counts/open").and_then(Value::as_u64),
            Some(0),
            "the only finding is directive-demoted; open bucket must be empty: {payload:#}",
        );
        assert_eq!(
            payload.pointer("/counts/ignored").and_then(Value::as_u64),
            Some(1),
            "the directive demotes UNI-100; ignored bucket must be 1: {payload:#}",
        );
        assert_eq!(payload.pointer("/counts/false_positive").and_then(Value::as_u64), Some(0));
        assert_eq!(payload.pointer("/exit_code").and_then(Value::as_i64), Some(0));
        assert!(
            payload.pointer("/duration_ms").and_then(Value::as_u64).is_some(),
            "duration_ms must be present and serialise as a JSON number: {payload:#}",
        );

        for forbidden in ["duration-ms", "false-positive", "exit-code"] {
            assert!(
                !last_line.contains(&format!("\"{forbidden}\"")),
                "lint-completed payload must use snake_case field names; raw:\n{last_line}",
            );
        }
    }

    /// rules-root resolution / lint exit mapping negative: with no `--rules-root`, no project-local
    /// `adapters/shared/rules/universal/` rung, and no distributed
    /// out-of-tree `<project-cache>/codex/` cache, the resolver returns
    /// `rules-root-required`. The CLI surfaces it on stderr and exits 2.
    #[test]
    fn review_missing_rules_root_exits_2() {
        let project_root = TempDir::new().expect("project tempdir");
        let project = project_root.path().join("project");
        fs::create_dir_all(project.join(".specify")).expect("mkdir project/.specify");
        fs::write(project.join(".specify").join("project.yaml"), "name: review-e2e-missing\n")
            .expect("write project.yaml");

        // Pass `--format json` so the failure envelope renders as JSON on
        // stderr (with the kebab-case `rule-id` field). The text branch
        // collapses to `error: validation failed: N errors` and would
        // hide the closed `rules-root-required` discriminant.
        let mut cmd = Command::cargo_bin("specify").expect("cargo_bin(specify)");
        cmd.arg("--format")
            .arg("json")
            .arg("lint")
            .arg("project")
            .arg("--target")
            .arg("omnia")
            .arg("--project-dir")
            .arg(&project)
            .arg("--output-format")
            .arg("json")
            .env_remove("RULES_ROOT");
        let output = cmd.output().expect("specify invocation");

        assert_eq!(
            output.status.code(),
            Some(2),
            "expected exit 2; stderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stderr = std::str::from_utf8(&output.stderr).expect("utf8 stderr");
        assert!(
            stderr.contains("rules-root-required"),
            "stderr must mention rules-root-required; got:\n{stderr}"
        );
    }

    /// Blocking-tier exit decision (non-blocking half): a `suggestion`-severity
    /// rule that matches still surfaces a finding on the envelope, but the scan
    /// exits `0` because only `critical | important` violations gate. Pins the
    /// `blocking` predicate (`crates/diagnostics/src/diagnostic.rs`) through the
    /// CLI boundary — today's tests only cover `important` -> exit 2 and the
    /// directive-demoted / `--dump-model` exit-0 paths, never a present-but-
    /// non-blocking finding.
    #[test]
    fn suggestion_finding_present_exits_0() {
        let fx = build_fixture();
        // Overwrite the default `important` UNI-100 with a `suggestion`-tier
        // rule matching the same `TODO` token in `notes.md`.
        write_regex_rule(&fx.codex, "UNI-100", "suggestion", "TODO");
        let output = run_review(&fx.project, Some(&fx.codex), &[]);

        assert_eq!(
            output.status.code(),
            Some(0),
            "a suggestion-tier finding is non-blocking; scan must exit 0; stderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = std::str::from_utf8(&output.stdout).expect("utf8 stdout");
        let envelope: Value = serde_json::from_str(stdout).expect("parse envelope");
        let suggestion = envelope
            .pointer("/summary/suggestion")
            .and_then(Value::as_u64)
            .expect("summary.suggestion present");
        assert!(
            suggestion >= 1,
            "the finding must still surface in the envelope, just non-blocking; envelope:\n{envelope:#}"
        );
        let important =
            envelope.pointer("/summary/important").and_then(Value::as_u64).unwrap_or_default();
        let critical =
            envelope.pointer("/summary/critical").and_then(Value::as_u64).unwrap_or_default();
        assert_eq!(
            important + critical,
            0,
            "no blocking-tier finding should exist; envelope:\n{envelope:#}"
        );
    }

    /// Blocking-tier exit decision (mixed half): with one `important` rule and
    /// one `suggestion` rule both matching, the scan exits `2` driven by the
    /// blocking tier — not by raw finding count. Proves the exit is severity-
    /// gated, complementing `suggestion_finding_present_exits_0`.
    #[test]
    fn blocking_tier_drives_exit() {
        let fx = build_fixture();
        // `build_fixture` already wrote the `important` UNI-100 (matches
        // `TODO`). Add a `suggestion` rule matching `scaffolding`, also
        // present in `notes.md` ("TODO: drop scaffolding.").
        write_regex_rule(&fx.codex, "UNI-101", "suggestion", "scaffolding");
        let output = run_review(&fx.project, Some(&fx.codex), &[]);

        assert_eq!(
            output.status.code(),
            Some(2),
            "the important finding must drive exit 2 despite a co-present suggestion; stderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = std::str::from_utf8(&output.stdout).expect("utf8 stdout");
        let envelope: Value = serde_json::from_str(stdout).expect("parse envelope");
        let important = envelope
            .pointer("/summary/important")
            .and_then(Value::as_u64)
            .expect("summary.important present");
        let suggestion = envelope
            .pointer("/summary/suggestion")
            .and_then(Value::as_u64)
            .expect("summary.suggestion present");
        assert!(
            important >= 1 && suggestion >= 1,
            "both tiers must surface (exit driven by the blocking tier, not count); envelope:\n{envelope:#}"
        );
    }
}
