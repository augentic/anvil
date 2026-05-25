use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;
use tooling::check::{
    check_recorded_trace_freshness, validate_scenario_frontmatter,
    SCENARIO_RULE_ARTIFACT_PATH_UNSAFE, SCENARIO_RULE_SCHEMA_VIOLATION, RULE_STAGES_NOT_CONTIGUOUS,
    RULE_RECORDED_TRACE_VIOLATION,
};
use tooling::Context;

fn scaffold_framework_root(base: &Path) -> PathBuf {
    let root = base.join("framework");
    fs::create_dir_all(root.join("tooling/schemas")).expect("schemas dir");
    fs::create_dir_all(root.join("plugins/spec")).expect("plugins dir");
    fs::create_dir_all(root.join("adapters/targets")).expect("targets dir");
    fs::write(root.join("tooling/Cargo.toml"), "stub").expect("tooling manifest");

    let schema_src = Path::new(env!("CARGO_MANIFEST_DIR")).join("schemas/scenario.schema.json");
    fs::copy(
        schema_src,
        root.join("tooling/schemas/scenario.schema.json"),
    )
    .expect("copy scenario schema");

    root
}

fn write_scenario(root: &Path, rel: &str, content: &str) -> PathBuf {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("scenario parent dir");
    }
    fs::write(&path, content).expect("write scenario");
    path
}

fn context_for(root: &Path) -> Context {
    Context::from_manifest_dir(root.join("tooling")).expect("framework root resolves")
}

const VALID_FRONTMATTER: &str = r#"---
id: demo-scenario
owner: demo
kind: suite
backend: manual
entrypoint: /spec:plan
stages: [plan]
isolation: fresh-project
---
"#;

#[test]
fn schema_violation_on_missing_required_field() {
    let tmp = TempDir::new().expect("tempdir");
    let root = scaffold_framework_root(tmp.path());
    write_scenario(
        &root,
        "tests/demo/scenario.md",
        &format!(
            "{VALID_FRONTMATTER}\n# Demo\n\nScenario ID: `demo-scenario`\n"
        )
        .replace("owner: demo\n", ""),
    );

    let findings = validate_scenario_frontmatter(&context_for(&root));
    let schema: Vec<_> = findings
        .iter()
        .filter(|f| f.rule_id == SCENARIO_RULE_SCHEMA_VIOLATION)
        .collect();
    assert!(!schema.is_empty(), "expected schema violation, got: {findings:?}");
    assert!(
        schema.iter().any(|f| f.message.contains("Scenario frontmatter:")),
        "expected Deno-shaped message prefix, got: {findings:?}"
    );
}

#[test]
fn stages_not_contiguous_prefix_emits_finding() {
    let tmp = TempDir::new().expect("tempdir");
    let root = scaffold_framework_root(tmp.path());
    write_scenario(
        &root,
        "tests/demo/scenario.md",
        &format!(
            "{VALID_FRONTMATTER}\n# Demo\n",
        )
        .replace("stages: [plan]", "stages: [plan, build]"),
    );

    let findings = validate_scenario_frontmatter(&context_for(&root));
    let stages: Vec<_> = findings
        .iter()
        .filter(|f| f.rule_id == RULE_STAGES_NOT_CONTIGUOUS)
        .collect();
    assert_eq!(stages.len(), 1, "expected one stages finding, got: {findings:?}");
    assert!(stages[0].message.contains("contiguous slice"));
}

#[test]
fn artifact_path_unsafe_rejects_parent_escape() {
    let tmp = TempDir::new().expect("tempdir");
    let root = scaffold_framework_root(tmp.path());
    write_scenario(
        &root,
        "tests/demo/scenario.md",
        &format!(
            "{VALID_FRONTMATTER}\n# Demo\n",
        )
        .replace(
            "isolation: fresh-project",
            "isolation: fresh-project\nexpected-artifacts:\n  - ../escape.yaml",
        ),
    );

    let findings = validate_scenario_frontmatter(&context_for(&root));
    let art: Vec<_> = findings
        .iter()
        .filter(|f| f.rule_id == SCENARIO_RULE_ARTIFACT_PATH_UNSAFE)
        .collect();
    assert_eq!(art.len(), 1, "expected one artifact finding, got: {findings:?}");
    assert!(art[0].message.contains("'..' segment not allowed"));
}

#[test]
fn recorded_trace_invalid_header_emits_violation() {
    let tmp = TempDir::new().expect("tempdir");
    let root = scaffold_framework_root(tmp.path());
    let trace = root.join("tests/recorded/demo.jsonl");
    fs::create_dir_all(trace.parent().unwrap()).expect("recorded dir");
    fs::write(&trace, r#"{"kind":"wrong"}"#).expect("bad trace");

    let findings = check_recorded_trace_freshness(&context_for(&root));
    assert!(
        findings.iter().any(|f| {
            f.rule_id == RULE_RECORDED_TRACE_VIOLATION
                && f.message.contains("kind must be 'recorded-trace-header'")
        }),
        "expected kind violation, got: {findings:?}"
    );
}
