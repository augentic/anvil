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
