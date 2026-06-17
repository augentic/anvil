use std::fs;
use std::path::Path;

use specify_diagnostics::blocking;
use tempfile::tempdir;

use super::{detect, platform_satisfied, source_materializable};
use crate::Platform;

fn write_project_yaml(root: &Path, platforms: &[&str]) {
    let yaml_platforms: Vec<String> = platforms.iter().map(|p| format!("  - {p}")).collect();
    let content = format!(
        "name: app-icon-gate-test\nadapter: vectis\nspecify_version: '{version}'\nplatforms:\n{platforms}",
        version = env!("CARGO_PKG_VERSION"),
        platforms = yaml_platforms.join("\n"),
    );
    let specify_dir = root.join(".specify");
    fs::create_dir_all(&specify_dir).expect("mkdir .specify");
    fs::write(specify_dir.join("project.yaml"), content).expect("write project.yaml");
}

fn seed_vectis_adapter(root: &Path) {
    let adapter = root.join("adapters/targets/vectis");
    let briefs = adapter.join("briefs");
    fs::create_dir_all(&briefs).expect("mkdir briefs");
    fs::write(
        adapter.join("adapter.yaml"),
        "name: vectis\nversion: 1.0.0\naxis: target\nexecution: agent\nbriefs:\n  shape: briefs/shape.md\n  build: briefs/build.md\n  merge: briefs/merge.md\ndescription: Test vectis adapter\n",
    )
    .expect("write adapter.yaml");
    for op in ["shape", "build", "merge"] {
        fs::write(
            briefs.join(format!("{op}.md")),
            format!("---\nid: {op}\ndescription: {op} brief\n---\n"),
        )
        .expect("write brief");
    }
}

fn bootstrap_fixture(platforms: &[&str]) -> tempfile::TempDir {
    let tmp = tempdir().expect("tempdir");
    write_project_yaml(tmp.path(), platforms);
    seed_vectis_adapter(tmp.path());
    tmp
}

fn write_assets(root: &Path, yaml: &str) {
    let design = root.join("design-system");
    fs::create_dir_all(design.join("assets")).expect("mkdir assets");
    fs::write(design.join("assets.yaml"), yaml).expect("write assets.yaml");
}

#[test]
fn greenfield_without_app_icon_emits_gate() {
    let fixture = bootstrap_fixture(&["core", "ios", "android"]);

    let hits: Vec<_> = detect(fixture.path())
        .into_iter()
        .filter(|d| d.rule_id.as_deref() == Some("plan-bootstrap-app-icon-missing"))
        .collect();
    assert_eq!(hits.len(), 2, "ios and android each need a finding: {hits:#?}");
    assert!(hits.iter().all(blocking));
    assert!(hits.iter().any(|d| d.impact.contains("ios")));
    assert!(hits.iter().any(|d| d.impact.contains("android")));
}

#[test]
fn valid_source_master_passes() {
    let fixture = bootstrap_fixture(&["core", "ios", "android"]);
    write_assets(
        fixture.path(),
        "version: 1\n\
         app-icon: app-icon\n\
         assets:\n\
         \x20\x20app-icon:\n\
         \x20\x20\x20\x20kind: vector\n\
         \x20\x20\x20\x20role: app-icon\n\
         \x20\x20\x20\x20source: assets/app-icon.svg\n",
    );
    fs::write(fixture.path().join("design-system/assets/app-icon.svg"), "<svg/>")
        .expect("write svg");

    let hits: Vec<_> = detect(fixture.path())
        .into_iter()
        .filter(|d| d.rule_id.as_deref() == Some("plan-bootstrap-app-icon-missing"))
        .collect();
    assert!(hits.is_empty(), "path A master should satisfy both platforms: {hits:#?}");
}

#[test]
fn invalid_assets_yaml_emits_gate() {
    let fixture = bootstrap_fixture(&["core", "ios", "android"]);
    write_assets(fixture.path(), "version: 1\napp-icon: app-icon\nassets: [\n");

    let hits: Vec<_> = detect(fixture.path())
        .into_iter()
        .filter(|d| d.rule_id.as_deref() == Some("plan-bootstrap-app-icon-missing"))
        .collect();
    assert_eq!(hits.len(), 2, "invalid YAML must not skip the gate: {hits:#?}");
    assert!(hits.iter().all(|d| d.impact.contains("not valid YAML")));
}

#[test]
fn shell_resident_ios_skips_design_system() {
    let fixture = bootstrap_fixture(&["core", "ios", "android"]);

    let appiconset = fixture.path().join("iOS/Demo/Resources/Assets.xcassets/AppIcon.appiconset");
    fs::create_dir_all(&appiconset).expect("mkdir appiconset");
    fs::write(
        appiconset.join("Contents.json"),
        r#"{"images":[{"filename":"AppIcon.png","idiom":"universal"}]}"#,
    )
    .expect("write contents");
    fs::write(appiconset.join("AppIcon.png"), minimal_png()).expect("write png");

    let hits: Vec<_> = detect(fixture.path())
        .into_iter()
        .filter(|d| d.rule_id.as_deref() == Some("plan-bootstrap-app-icon-missing"))
        .collect();
    assert_eq!(hits.len(), 1, "android still missing app-icon: {hits:#?}");
    assert!(hits[0].impact.contains("android"));
}

#[test]
fn incremental_shells_present_no_gate() {
    let fixture = bootstrap_fixture(&["core", "ios", "android"]);
    fs::create_dir_all(fixture.path().join("shared/src")).expect("mkdir core");
    fs::write(fixture.path().join("shared/src/app.rs"), "fn app() {}\n").expect("write core");
    let ios = fixture.path().join("iOS/Demo");
    fs::create_dir_all(&ios).expect("mkdir ios");
    fs::write(ios.join("App.swift"), "import SwiftUI\n").expect("write swift");
    let android = fixture.path().join("Android/app/src/main/java/demo");
    fs::create_dir_all(&android).expect("mkdir android");
    fs::write(android.join("Main.kt"), "fun main() {}\n").expect("write kt");

    assert!(detect(fixture.path()).is_empty(), "complete shells must not trigger §6.1");
}

#[test]
fn path_b_pin_satisfies_platform() {
    let tmp = tempdir().expect("tempdir");
    let assets_dir = tmp.path().join("design-system/assets");
    fs::create_dir_all(assets_dir.join("exports/ios/app-icon/AppIcon.appiconset")).expect("mkdir");
    let entry = serde_json::json!({
        "kind": "vector",
        "role": "app-icon",
        "sources": { "ios": "exports/ios/app-icon/AppIcon.appiconset" }
    });
    assert!(platform_satisfied(&assets_dir, &entry, Platform::Ios));
}

#[test]
fn source_kind_mismatch() {
    let tmp = tempdir().expect("tempdir");
    let assets_dir = tmp.path().join("assets");
    fs::create_dir_all(&assets_dir).expect("mkdir");
    fs::write(assets_dir.join("icon.png"), b"png").expect("write png");
    let entry = serde_json::json!({ "kind": "vector", "role": "app-icon", "source": "icon.png" });
    assert!(!source_materializable(&assets_dir, &entry, "icon.png"));
}

fn minimal_png() -> Vec<u8> {
    vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F,
        0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00,
        0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
        0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ]
}
