use std::fs;
use std::path::{Path, PathBuf};

use tooling::check::{
    SkillBodyLineCount, SkillEnvelopeJsonInBody, SkillFrontmatterRestatement,
    SkillInlineJsonTooLong, SkillInvalidCriticalPath, SkillMissingCriticalPath,
    SkillSectionLineCount, SkillStepBodyDuplicatesCriticalPath, SkillVariableCoverage,
};
use tooling::finding::Check;
use tooling::Context;

fn fixture_root(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/skill_body")
        .join(name)
}

fn scaffold_framework_root(root: &Path) {
    fs::create_dir_all(root.join("plugins/demo/skills/test")).expect("skill dir");
    fs::create_dir_all(root.join("adapters")).expect("adapters dir");
    fs::create_dir_all(root.join("tooling")).expect("tooling dir");
    fs::write(root.join("tooling/Cargo.toml"), "").expect("tooling manifest");
}

fn write_skill(root: &Path, body: &str) {
    let content = format!(
        "---\nname: test-skill\ndescription: Fixture skill for body discipline checks in tooling tests.\nargument-hint: <arg>\n---\n\n{body}\n"
    );
    fs::write(
        root.join("plugins/demo/skills/test/SKILL.md"),
        content,
    )
    .expect("write skill");
}

fn context_for_fixture(name: &str) -> Context {
    let root = fixture_root(name);
    scaffold_framework_root(&root);
    Context::from_manifest_dir(root.join("tooling")).expect("framework root resolves")
}

fn repeated_lines(prefix: &str, count: usize) -> String {
    (0..count)
        .map(|i| format!("{prefix} {i}"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn body_line_count_flags_long_body() {
    let ctx = context_for_fixture("body-too-long");
    write_skill(&fixture_root("body-too-long"), &repeated_lines("line", 201));

    let findings = SkillBodyLineCount.run(&ctx);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "skill.body-line-count");
    assert!(findings[0].message.contains("body lines (limit 200)"));
    assert!(findings[0].message.contains("Skill body too long"));
}

#[test]
fn body_line_count_passes_within_cap() {
    let ctx = context_for_fixture("body-ok");
    write_skill(&fixture_root("body-ok"), &repeated_lines("line", 10));

    let findings = SkillBodyLineCount.run(&ctx);
    assert!(findings.is_empty());
}

#[test]
fn section_line_count_flags_long_h2() {
    let ctx = context_for_fixture("section-too-long");
    let body = format!(
        "## Section\n\n{}\n\n## Other\n\nok",
        repeated_lines("item", 46)
    );
    write_skill(&fixture_root("section-too-long"), &body);

    let findings = SkillSectionLineCount.run(&ctx);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "skill.section-line-count");
    assert!(findings[0].message.contains("'Section' (46 lines)"));
}

#[test]
fn missing_critical_path_flags_long_skill_without_heading() {
    let ctx = context_for_fixture("missing-critical-path");
    write_skill(
        &fixture_root("missing-critical-path"),
        &repeated_lines("line", 150),
    );

    let findings = SkillMissingCriticalPath.run(&ctx);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "skill.missing-critical-path");
    assert!(findings[0].message.contains("Missing Critical Path"));
}

#[test]
fn invalid_critical_path_flags_wrong_item_count() {
    let ctx = context_for_fixture("invalid-critical-path");
    let mut body = String::from("## Critical Path\n\n");
    for i in 1..=4 {
        body.push_str(&format!("{i}. Step {i}\n"));
    }
    body.push('\n');
    body.push_str(&repeated_lines("padding", 150));
    write_skill(&fixture_root("invalid-critical-path"), &body);

    let findings = SkillInvalidCriticalPath.run(&ctx);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "skill.invalid-critical-path");
    assert!(findings[0].message.contains("found 4"));
}

#[test]
fn inline_json_too_long_flags_large_fence() {
    let ctx = context_for_fixture("inline-json-too-long");
    let mut body = String::from("## Example\n\n```json\n");
    body.push_str(&repeated_lines("\"k\": \"v\"", 31));
    body.push_str("\n```\n");
    write_skill(&fixture_root("inline-json-too-long"), &body);

    let findings = SkillInlineJsonTooLong.run(&ctx);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "skill.inline-json-too-long");
    assert!(findings[0].message.contains("31 body lines"));
}

#[test]
fn envelope_json_in_body_flags_envelope_shape() {
    let ctx = context_for_fixture("envelope-json");
    let body = r##"## Output

```json
{
  "envelope-version": "1",
  "ok": true,
  "data": {}
}
```
"##;
    write_skill(&fixture_root("envelope-json"), body);

    let findings = SkillEnvelopeJsonInBody.run(&ctx);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "skill.envelope-json-in-body");
    assert!(findings[0].message.contains("Envelope JSON in skill body"));
}

#[test]
fn step_body_duplicates_critical_path_flags_match() {
    let ctx = context_for_fixture("step-duplicate");
    let body = r#"## Critical Path

1. Run the validator

## Steps

1. Run the validator
"#;
    write_skill(&fixture_root("step-duplicate"), body);

    let findings = SkillStepBodyDuplicatesCriticalPath.run(&ctx);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "skill.step-body-duplicates-critical-path");
    assert!(findings[0].message.contains("Step body duplicates Critical Path"));
}

#[test]
fn frontmatter_restatement_flags_input_heading() {
    let ctx = context_for_fixture("frontmatter-restatement");
    write_skill(
        &fixture_root("frontmatter-restatement"),
        "## Input\n\nProvide the slice name.\n",
    );

    let findings = SkillFrontmatterRestatement.run(&ctx);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "skill.frontmatter-restatement");
    assert!(findings[0].message.contains("## Input"));
}

#[test]
fn variable_coverage_flags_unused_definition() {
    let ctx = context_for_fixture("unused-variable");
    let body = r#"## Arguments

```text
$SLICE=<name>
```

## Steps

Use the operator-provided name directly.
"#;
    write_skill(&fixture_root("unused-variable"), body);

    let findings = SkillVariableCoverage.run(&ctx);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "skill.variable-coverage");
    assert!(findings[0].message.contains("Unused variable"));
    assert!(findings[0].message.contains("$SLICE"));
}

#[test]
fn variable_coverage_flags_undefined_use() {
    let ctx = context_for_fixture("undefined-variable");
    let body = r#"## Arguments

```text
$SLICE=<name>
```

## Steps

Validate $PROJECT for $SLICE before continuing.
"#;
    write_skill(&fixture_root("undefined-variable"), body);

    let findings = SkillVariableCoverage.run(&ctx);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "skill.variable-coverage");
    assert!(findings[0].message.contains("Undefined variable"));
    assert!(findings[0].message.contains("$PROJECT"));
}

#[test]
fn skill_body_checks_pass_on_real_repo() {
    let ctx = Context::from_manifest_dir(env!("CARGO_MANIFEST_DIR")).expect("framework root");
    let checks: [&dyn Check; 9] = [
        &SkillBodyLineCount,
        &SkillSectionLineCount,
        &SkillMissingCriticalPath,
        &SkillInvalidCriticalPath,
        &SkillInlineJsonTooLong,
        &SkillEnvelopeJsonInBody,
        &SkillStepBodyDuplicatesCriticalPath,
        &SkillFrontmatterRestatement,
        &SkillVariableCoverage,
    ];
    let mut findings = Vec::new();
    for check in checks {
        findings.extend(check.run(&ctx));
    }
    assert!(
        findings.is_empty(),
        "expected clean skill body checks, got: {findings:?}"
    );
}
