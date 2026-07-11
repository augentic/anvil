//! Integration coverage for shallow root-marker detection
//! (`workflow::agents::detect::detect_root_markers`): deterministic
//! runtime ordering and the corrupt-marker warning path, asserted
//! through the public bullet projections and warning fields.

use std::fs;

use tempfile::tempdir;
use workflow::agents::detect::detect_root_markers;

#[test]
fn mixed_runtime_order_is_deterministic() {
    let tmp = tempdir().expect("tempdir");
    fs::write(tmp.path().join("package.json"), r#"{"engines":{"node":">=20"}}"#).expect("package");
    fs::write(tmp.path().join("go.mod"), "module demo\n\ngo 1.22\n").expect("go");
    fs::write(tmp.path().join("Cargo.toml"), "[package]\nname = \"demo\"\n").expect("cargo");

    let detection = detect_root_markers(tmp.path());

    assert_eq!(
        detection.runtime_bullets(),
        vec![
            "detected: Go 1.22.".to_string(),
            "detected: Node.js (engines.node `>=20`).".to_string(),
            "detected: Rust.".to_string(),
        ]
    );
}

#[test]
fn corrupt_markers_warn_not_detect() {
    let tmp = tempdir().expect("tempdir");
    fs::write(tmp.path().join("Cargo.toml"), "package = [").expect("cargo");
    fs::write(tmp.path().join("package.json"), "{").expect("package");
    fs::create_dir_all(tmp.path().join(".github/workflows")).expect("workflows dir");
    fs::write(tmp.path().join(".github/workflows/ci.yaml"), "name: [").expect("workflow");

    let detection = detect_root_markers(tmp.path());

    assert!(detection.runtimes.is_empty());
    assert_eq!(
        detection.warnings.iter().map(|warning| warning.path.as_str()).collect::<Vec<_>>(),
        vec![".github/workflows/ci.yaml", "Cargo.toml", "package.json"]
    );
}
