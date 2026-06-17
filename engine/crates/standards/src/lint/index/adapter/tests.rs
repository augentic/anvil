use super::*;
use crate::lint::FileKind;

fn manifest(relative: &str, body: &str) -> DiscoveredFile {
    DiscoveredFile {
        relative: relative.into(),
        kind: FileKind::Text,
        language: Some("yaml".into()),
        bytes: Some(body.as_bytes().to_vec()),
    }
}

#[test]
fn extracts_source_manifest() {
    let file = manifest(
        "adapters/sources/intent/adapter.yaml",
        "name: intent\nversion: \"1.0.0\"\naxis: source\n",
    );
    let manifest = extract(&file).expect("manifest extracted");
    assert_eq!(manifest.axis, AdapterAxis::Sources);
    assert_eq!(manifest.name, "intent");
    assert_eq!(manifest.version.as_deref(), Some("1.0.0"));
    assert!(manifest.brief_keys.is_empty());
}

#[test]
fn extracts_target_manifest_string_version() {
    let file = manifest("adapters/targets/omnia/adapter.yaml", "name: omnia\nversion: \"2.1\"\n");
    let manifest = extract(&file).expect("manifest extracted");
    assert_eq!(manifest.axis, AdapterAxis::Targets);
    assert_eq!(manifest.version.as_deref(), Some("2.1"));
}

#[test]
fn extracts_briefs_keys_when_declared() {
    let file = manifest(
        "adapters/sources/intent/adapter.yaml",
        "name: intent\nversion: \"1.0.0\"\nbriefs:\n  survey: briefs/survey.md\n  extract: briefs/extract.md\n",
    );
    let manifest = extract(&file).expect("manifest extracted");
    assert_eq!(manifest.brief_keys, vec!["extract".to_string(), "survey".to_string()]);
}

#[test]
fn missing_briefs_leaves_keys_empty() {
    let file = manifest("adapters/targets/omnia/adapter.yaml", "name: omnia\nversion: \"1.0.0\"\n");
    let manifest = extract(&file).expect("manifest extracted");
    assert!(manifest.brief_keys.is_empty());
}

#[test]
fn rejects_unknown_axis() {
    let file = manifest("adapters/whatever/intent/adapter.yaml", "name: intent\n");
    assert!(extract(&file).is_none());
}

#[test]
fn rejects_nested_adapter_yaml() {
    let file = manifest("adapters/sources/intent/briefs/adapter.yaml", "name: nope\n");
    assert!(extract(&file).is_none());
}
