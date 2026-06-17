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
    "https://github.com/augentic/specify-cli/schemas/diagnostics/diagnostic.schema.json";

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
    let schema: Value = serde_json::from_str(WORKSPACE_MODEL_JSON_SCHEMA).expect("parse schema");
    jsonschema::validator_for(&schema).expect("validator build")
}

#[track_caller]
fn assert_validates(validator: &Validator, stdout: &str, schema_label: &str) {
    let instance: Value = serde_json::from_str(stdout)
        .unwrap_or_else(|err| panic!("stdout is not JSON ({err}); raw:\n{stdout}"));
    let errors: Vec<String> = validator.iter_errors(&instance).map(|err| err.to_string()).collect();
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
