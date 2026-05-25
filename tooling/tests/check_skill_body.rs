use std::fs;
use std::path::{Path, PathBuf};

use tooling::check::{
    SkillBodyLineCount, SkillEnvelopeJsonInBody, SkillInvalidCriticalPath, SkillVariableCoverage,
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
