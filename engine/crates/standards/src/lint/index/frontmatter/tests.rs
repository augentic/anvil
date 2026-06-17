use super::*;
use crate::lint::FileKind;

fn markdown(text: &str) -> DiscoveredFile {
    DiscoveredFile {
        relative: "doc.md".into(),
        kind: FileKind::Text,
        language: Some("markdown".into()),
        bytes: Some(text.as_bytes().to_vec()),
    }
}

#[test]
fn parses_simple_frontmatter() {
    let f = markdown("---\ntitle: Demo\nschema_id: rule\n---\n# Body\n");
    let fm = extract(&f).expect("frontmatter present");
    assert_eq!(fm.path, "doc.md");
    assert_eq!(fm.schema_id, None);
    assert_eq!(fm.fields.get("title").and_then(Value::as_str), Some("Demo"));
    assert_eq!(fm.fields.get("schema_id").and_then(Value::as_str), Some("rule"));
}

#[test]
fn non_markdown_returns_none() {
    let f = DiscoveredFile {
        relative: "src/lib.rs".into(),
        kind: FileKind::Text,
        language: Some("rust".into()),
        bytes: Some(b"// some code".to_vec()),
    };
    assert!(extract(&f).is_none());
}

#[test]
fn missing_frontmatter_returns_none() {
    let f = markdown("# heading only\n");
    assert!(extract(&f).is_none());
}

#[test]
fn invalid_yaml_returns_none() {
    let f = markdown("---\n  : : :\n---\nbody\n");
    assert!(extract(&f).is_none());
}
