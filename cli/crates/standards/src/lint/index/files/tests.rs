use super::*;

#[test]
fn binary_sniff_detects_nul_byte() {
    let bytes = b"hello\x00world".to_vec();
    let (kind, carried) = classify(&bytes);
    assert_eq!(kind, FileKind::Binary);
    assert!(carried.is_none());
}

#[test]
fn binary_sniff_passes_text() {
    let bytes = b"plain text body\n".to_vec();
    let (kind, carried) = classify(&bytes);
    assert_eq!(kind, FileKind::Text);
    assert_eq!(carried.as_deref(), Some(b"plain text body\n".as_slice()));
}

#[test]
fn include_filter_accepts_dot_specify_paths() {
    assert!(is_included(".specify/slices/foo/spec.md"));
}

#[test]
fn include_filter_accepts_known_extensions() {
    assert!(is_included("src/lib.rs"));
    assert!(is_included("README.md"));
    assert!(is_included("docs/index.md"));
    assert!(is_included("project.yaml"));
}

#[test]
fn rejects_unknown_extensions() {
    assert!(!is_included("photo.png"));
    assert!(!is_included("data.bin"));
    assert!(!is_included("notes.txt"));
    assert!(!is_included("no-extension"));
}

#[test]
fn language_inference_covers_extensions() {
    assert_eq!(infer_language("src/lib.rs").as_deref(), Some("rust"));
    assert_eq!(infer_language("App.swift").as_deref(), Some("swift"));
    assert_eq!(infer_language("build.gradle").as_deref(), Some("kotlin"));
    assert_eq!(infer_language("README.md").as_deref(), Some("markdown"));
    assert_eq!(infer_language("data.json").as_deref(), Some("json"));
    assert_eq!(infer_language("notes.txt"), None);
}
