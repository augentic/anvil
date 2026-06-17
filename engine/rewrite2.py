import re

with open('crates/model/tests/validate.rs', 'r') as f:
    content = f.read()

# For skips_metadata_free_blocks
old_1 = r'''#[test]
fn skips_metadata_free_blocks() {.*?let assert = specify_cmd\(\).*?\.assert\(\);.*?for finding in findings \{.*?let rule_id = finding\["rule-id"\].as_str\(\).unwrap_or\(""\);.*?assert!\(.*?\!rule_id\.starts_with\("spec\.requirement-"\),.*?got: \{rule_id\}"\n            \);\n        \}\n    \}\n}'''
new_1 = '''#[test]
fn skips_metadata_free_blocks() {
    let spec = "### Requirement: metadata-free body\\n\\n\\
                ID: REQ-001\\n\\n\\
                body that has no Sources or Status yet\\n";
    let project = stage_slice_with_spec(spec, None);
    let slice_dir = project.root().join(".specify/slices/my-slice");
    let diagnostics = specify_model::validate::validate_slice(&slice_dir).unwrap_or_default();
    for finding in diagnostics {
        let rule_id = finding.rule_id.as_deref().unwrap_or("");
        assert!(
            !rule_id.starts_with("spec.requirement-"),
            "no provenance rule should fire on a metadata-free spec.md, got: {rule_id}"
        );
    }
}'''

content = re.sub(old_1, new_1, content, flags=re.DOTALL)

# For flags_thin_synopsis_non_blocking
old_2 = r'''#[test]
fn flags_thin_synopsis_non_blocking\(\) \{.*?let project = Project::init\(\);.*?specify_cmd\(\).*?\.success\(\);.*?let assert = specify_cmd\(\).*?\.assert\(\);.*?let report = parse_json\(&assert\.get_output\(\)\.stdout\);.*?let findings = report\["findings"\].as_array\(\).expect\("findings array"\);.*?let target = findings.*?\.find\(\|r\| r\["rule-id"\] == "discovery-lead-synopsis-thin"\).*?\.expect\("finding exists"\);.*?assert_eq!\(target\["severity"\], "suggestion"\);.*?assert!\(.*?"docs:identity-api"\).*?assert!\(.*?!"legacy:identity-api"\).*?\}'''

new_2 = '''#[test]
fn flags_thin_synopsis_non_blocking() {
    let project = Project::new().with_schemas();
    let slice_dir = project.root().join(".specify/slices/my-slice");
    std::fs::create_dir_all(&slice_dir).unwrap();

    let discovery = "\\
# Discovery — identity

## Lead inventory

### Source: legacy
- `legacy:identity-api`
  This lead has a fully populated, multi-line synopsis that
  meaningfully explains what the agent found in the source.

### Source: docs
- `docs:identity-api`
  Identity-api (thin echo without content).
";
    std::fs::write(project.root().join("discovery.md"), discovery).unwrap();

    let diagnostics = specify_model::validate::validate_slice(&slice_dir).unwrap_or_default();
    
    let target = diagnostics
        .iter()
        .find(|r| r.rule_id.as_deref() == Some("discovery-lead-synopsis-thin"))
        .expect("finding exists");
    
    assert_eq!(target.severity, specify_diagnostics::Severity::Suggestion);
    assert!(target.impact.contains("docs:identity-api"));
    assert!(!target.impact.contains("legacy:identity-api"));
}'''

content = re.sub(old_2, new_2, content, flags=re.DOTALL)

with open('crates/model/tests/validate.rs', 'w') as f:
    f.write(content)
