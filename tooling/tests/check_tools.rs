use std::fs;
use std::path::{Path, PathBuf};

use tooling::check::tools::{
    run_declared_tool_equivalent_invocations, run_first_party_tool_declarations,
};
use tooling::Context;

fn scaffold_framework_root(root: &Path) -> PathBuf {
    fs::create_dir_all(root.join("tooling")).expect("tooling dir");
    fs::write(root.join("tooling/Cargo.toml"), "[package]\nname = \"tooling\"\n").expect("cargo.toml");
    fs::create_dir_all(root.join("plugins")).expect("plugins dir");
    fs::create_dir_all(root.join("adapters/targets")).expect("targets dir");
    root.to_path_buf()
}

fn write_adapter(root: &Path, adapter: &str, contents: &str) {
    let dir = root.join("adapters/targets").join(adapter);
    fs::create_dir_all(&dir).expect("adapter dir");
    fs::write(dir.join("adapter.yaml"), contents).expect("adapter.yaml");
}

fn valid_adapters(root: &Path) {
    write_adapter(
        root,
        "contracts",
        "tools:\n  - name: contract\n    version: 0.3.0\n",
    );
    write_adapter(
        root,
        "vectis",
        "tools:\n  - name: vectis\n    version: 0.3.0\n",
    );
}

fn ctx_for(root: &Path) -> Context {
    Context::from_framework_root(root).expect("framework root")
}

#[test]
fn valid_first_party_declarations_pass() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = scaffold_framework_root(tmp.path());
    valid_adapters(&root);

    let findings = run_first_party_tool_declarations(&ctx_for(&root));
    assert!(findings.is_empty(), "expected no findings: {findings:?}");
}

#[test]
fn missing_tool_fails() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = scaffold_framework_root(tmp.path());
    write_adapter(&root, "contracts", "tools: []\n");
    write_adapter(
        &root,
        "vectis",
        "tools:\n  - name: vectis\n    version: 0.3.0\n",
    );

    let findings = run_first_party_tool_declarations(&ctx_for(&root));
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "tools.invalid-declaration");
    assert!(findings[0].message.contains("missing tool 'contract'"));
}

#[test]
fn wrong_package_version_fails() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = scaffold_framework_root(tmp.path());
    write_adapter(
        &root,
        "contracts",
        "tools:\n  - name: contract\n    version: 0.2.0\n",
    );
    write_adapter(
        &root,
        "vectis",
        "tools:\n  - name: vectis\n    version: 0.3.0\n",
    );

    let findings = run_first_party_tool_declarations(&ctx_for(&root));
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "tools.invalid-declaration");
    assert!(findings[0].message.contains("package must be 'specify:contract@0.3.0'"));
}

#[test]
fn invalid_tool_entry_shape_fails() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = scaffold_framework_root(tmp.path());
    write_adapter(
        &root,
        "contracts",
        "tools:\n  - not-an-object\n  - name: contract\n    version: 0.3.0-rc.1\n",
    );
    write_adapter(
        &root,
        "vectis",
        "tools:\n  - name: vectis\n    version: 0.3.0\n",
    );

    let findings = run_first_party_tool_declarations(&ctx_for(&root));
    assert_eq!(findings.len(), 3);
    assert!(findings.iter().all(|f| f.rule_id == "tools.invalid-declaration"));
    assert!(
        findings
            .iter()
            .any(|f| f.message.contains("must be { name, version } objects"))
    );
    assert!(
        findings
            .iter()
            .any(|f| f.message.contains("without prerelease metadata"))
    );
}

#[test]
fn retired_helper_in_brief_fails() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = scaffold_framework_root(tmp.path());
    valid_adapters(&root);
    let brief = root
        .join("adapters/targets/contracts/briefs/build.md");
    fs::create_dir_all(brief.parent().unwrap()).expect("brief dir");
    fs::write(&brief, "Run specify-contract on the baseline.\n").expect("brief");

    let findings = run_declared_tool_equivalent_invocations(&ctx_for(&root));
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "tools.invocation-not-equivalent");
    assert!(findings[0].message.contains("specify-contract"));
    assert!(findings[0].message.contains("specify tool run contract"));
}

#[test]
fn retired_helper_in_skill_fails() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let root = scaffold_framework_root(tmp.path());
    valid_adapters(&root);
    let skill = root.join("plugins/spec/skills/build/SKILL.md");
    fs::create_dir_all(skill.parent().unwrap()).expect("skill dir");
    fs::write(&skill, "Use specify-vectis validate on the tree.\n").expect("skill");

    let findings = run_declared_tool_equivalent_invocations(&ctx_for(&root));
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "tools.invocation-not-equivalent");
    assert!(findings[0].message.contains("specify-vectis validate"));
}

#[test]
fn real_repo_tools_checks_pass() {
    let ctx = Context::from_manifest_dir(env!("CARGO_MANIFEST_DIR")).expect("framework root");
    let decl = run_first_party_tool_declarations(&ctx);
    let invoke = run_declared_tool_equivalent_invocations(&ctx);
    assert!(decl.is_empty(), "declarations should pass: {decl:?}");
    assert!(invoke.is_empty(), "invocations should pass: {invoke:?}");
}
