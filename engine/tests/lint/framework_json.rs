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

use crate::support::scaffold_framework;

/// Replacement token for the canonicalised framework-root prefix in
/// every captured `location.path`. Chosen so it cannot occur in a
/// real filesystem path.
const FRAMEWORK_ROOT_PLACEHOLDER: &str = "<FRAMEWORK_ROOT>";

/// Resolve the directory where golden fixtures live.
fn goldens_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures").join("lint-framework")
}

/// The shared `crate::support::scaffold_framework` base tree plus the
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
    fs::write(&bad_manifest, "name: omnia\nversion: 1.0.0\naxis: target\n").expect("bad manifest");
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
