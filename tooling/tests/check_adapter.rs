use std::fs;
use std::path::{Path, PathBuf};

use tooling::check::{run_adapter_check, RULE_MISSING_MANIFEST, RULE_SCHEMA_VIOLATION};
use tooling::context::specify_cli_schemas_available;
use tooling::Context;

fn scaffold_framework(root: &Path) {
    fs::create_dir_all(root.join("plugins")).expect("plugins");
    fs::create_dir_all(root.join("adapters/sources")).expect("sources");
    fs::create_dir_all(root.join("adapters/targets")).expect("targets");
    fs::create_dir_all(root.join("tooling")).expect("tooling");
    fs::write(root.join("tooling/Cargo.toml"), "[package]\nname = \"tooling\"\n")
        .expect("cargo.toml");
}

fn real_framework_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .expect("framework root from tooling/Cargo.toml")
}

/// Absolute `specify-cli` checkout for adapter schema validation in tests.
///
/// CI sparse-checkouts schemas at `<framework-root>/specify-cli`; local dev usually
/// keeps a sibling checkout at `<parent-of-framework-root>/specify-cli`. Relative
/// `SPECIFY_CLI_DIR` values are resolved against the real framework root, not a
/// temp scaffold, so unit tests stay aligned with `make check` / CI.
fn absolute_specify_cli_dir() -> PathBuf {
    let framework_root = real_framework_root();

    if let Ok(value) = std::env::var("SPECIFY_CLI_DIR") {
        let candidate = PathBuf::from(&value);
        let resolved = if candidate.is_absolute() {
            candidate
        } else {
            framework_root.join(candidate)
        };
        if specify_cli_schemas_available(&resolved) {
            return resolved
                .canonicalize()
                .unwrap_or(resolved);
        }
    }

    for candidate in [
        framework_root.join("specify-cli"),
        framework_root
            .parent()
            .map(|parent| parent.join("specify-cli"))
            .unwrap_or_else(|| framework_root.join("specify-cli")),
    ] {
        if specify_cli_schemas_available(&candidate) {
            return candidate
                .canonicalize()
                .expect("specify-cli checkout with schemas/");
        }
    }

    panic!(
        "specify-cli checkout with schemas/ not found — clone https://github.com/augentic/specify-cli to ../specify-cli (sibling of this repo), sparse-checkout schemas/ at specify-cli/, or set SPECIFY_CLI_DIR"
    );
}

#[test]
fn schema_violation_on_invalid_source_manifest() {
    let temp = tempfile::tempdir().expect("tempdir");
    scaffold_framework(temp.path());

    let adapter_dir = temp.path().join("adapters/sources/bad-source");
    fs::create_dir_all(&adapter_dir).expect("adapter dir");
    fs::write(
        adapter_dir.join("adapter.yaml"),
        "name: bad-source\nversion: 1\naxis: source\n",
    )
    .expect("write manifest");

    let ctx = Context::from_framework_root(temp.path()).expect("context");
    std::env::set_var("SPECIFY_CLI_DIR", absolute_specify_cli_dir());

    let findings = run_adapter_check(&ctx);
    std::env::remove_var("SPECIFY_CLI_DIR");

    let schema_findings: Vec<_> = findings
        .iter()
        .filter(|finding| finding.rule_id == RULE_SCHEMA_VIOLATION)
        .collect();
    assert!(
        !schema_findings.is_empty(),
        "expected schema violation findings, got: {findings:?}"
    );
    assert!(
        schema_findings
            .iter()
            .any(|finding| finding.message.contains("Adapter validation failed:")),
        "expected Deno-shaped adapter validation message, got: {findings:?}"
    );
    assert!(
        schema_findings
            .iter()
            .any(|finding| finding.message.contains("missing required property")),
        "expected missing required property detail, got: {findings:?}"
    );
}

#[test]
fn missing_manifest_on_adapter_directory_without_yaml() {
    let temp = tempfile::tempdir().expect("tempdir");
    scaffold_framework(temp.path());

    fs::create_dir_all(temp.path().join("adapters/sources/no-manifest")).expect("adapter dir");

    let ctx = Context::from_framework_root(temp.path()).expect("context");
    let findings = run_adapter_check(&ctx);

    let missing: Vec<_> = findings
        .iter()
        .filter(|finding| finding.rule_id == RULE_MISSING_MANIFEST)
        .collect();
    assert_eq!(missing.len(), 1, "expected one missing-manifest finding");
    assert!(missing[0].message.contains("adapters/sources/no-manifest"));
    assert!(missing[0].message.contains("adapter.yaml"));
}
