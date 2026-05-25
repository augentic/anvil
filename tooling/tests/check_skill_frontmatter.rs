use std::fs;
use std::path::Path;

use tooling::check::{
    SkillArgumentHintGrammarCheck, SkillDescriptionGrammarCheck, SkillFrontmatterSchemaCheck,
    SkillNameDirectoryMismatchCheck, SkillUnknownToolCheck, RULE_UNKNOWN_TOOL,
    SKILL_RULE_SCHEMA_VIOLATION,
};
use tooling::finding::Check;
use tooling::Context;

fn fixture_context(root: &Path) -> Context {
    Context::from_manifest_dir(root.join("tooling")).expect("fixture framework root")
}

fn write_framework_scaffold(root: &Path) {
    fs::create_dir_all(root.join("adapters")).expect("adapters dir");
    fs::create_dir_all(root.join("plugins")).expect("plugins dir");
    fs::create_dir_all(root.join("tooling").join("schemas")).expect("schemas dir");
    fs::write(root.join("tooling").join("Cargo.toml"), "[package]\nname = \"tooling\"\n")
        .expect("tooling manifest");
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("schemas/skill.schema.json"),
        root.join("tooling/schemas/skill.schema.json"),
    )
    .expect("copy skill schema");
}

fn write_skill(root: &Path, plugin: &str, skill: &str, frontmatter: &str) {
    let dir = root
        .join("plugins")
        .join(plugin)
        .join("skills")
        .join(skill);
    fs::create_dir_all(&dir).expect("skill dir");
    fs::write(
        dir.join("SKILL.md"),
        format!("---\n{frontmatter}\n---\n\n# Test\n"),
    )
    .expect("skill md");
}

#[test]
fn schema_check_reports_missing_use_when_clause() {
    let temp = tempfile::tempdir().expect("temp dir");
    write_framework_scaffold(temp.path());
    write_skill(
        temp.path(),
        "demo",
        "bad-description",
        "name: demo-bad-description\ndescription: Too short.",
    );

    let ctx = fixture_context(temp.path());
    let findings = SkillFrontmatterSchemaCheck.run(&ctx);
    assert!(
        findings.iter().any(|finding| {
            finding.rule_id == SKILL_RULE_SCHEMA_VIOLATION
                && finding.message.contains("bad-description")
                && finding.message.contains("/description")
        }),
        "expected schema violation for description, got {findings:?}"
    );
}

#[test]
fn unknown_tool_check_reports_disallowed_tool() {
    let temp = tempfile::tempdir().expect("temp dir");
    write_framework_scaffold(temp.path());
    write_skill(
        temp.path(),
        "demo",
        "bad-tools",
        "name: demo-bad-tools\ndescription: Build demo fixtures for tool validation. Use when checking allowed-tools whitelisting.\nallowed-tools: NotARealTool",
    );

    let ctx = fixture_context(temp.path());
    let findings = SkillUnknownToolCheck.run(&ctx);
    assert!(
        findings.iter().any(|finding| {
            finding.rule_id == RULE_UNKNOWN_TOOL
                && finding.message.contains("NotARealTool")
        }),
        "expected unknown-tool finding, got {findings:?}"
    );
}

#[test]
fn spec_prefix_override_accepts_specify_prefix() {
    let temp = tempfile::tempdir().expect("temp dir");
    write_framework_scaffold(temp.path());
    write_skill(
        temp.path(),
        "spec",
        "init",
        "name: specify-init\ndescription: Initialize Specify in a project. Use when first wiring up a project before any other slash command.\nargument-hint: <adapter>",
    );

    let ctx = fixture_context(temp.path());
    assert!(SkillFrontmatterSchemaCheck.run(&ctx).is_empty());
    assert!(SkillNameDirectoryMismatchCheck.run(&ctx).is_empty());
    assert!(SkillDescriptionGrammarCheck.run(&ctx).is_empty());
    assert!(SkillArgumentHintGrammarCheck.run(&ctx).is_empty());
}
