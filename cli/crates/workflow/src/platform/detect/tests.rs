//! Unit tests for [`super::vectis_missing_platforms`].

use std::fs;
use std::path::Path;

use tempfile::tempdir;

use super::vectis_missing_platforms;
use crate::Platform;

const ALL_SUPPORTED: [Platform; 3] = [Platform::Core, Platform::Ios, Platform::Android];

fn write_project_yaml(root: &Path, adapter: &str, platforms: &[&str]) {
    let yaml_platforms: Vec<String> = platforms.iter().map(|p| format!("  - {p}")).collect();
    let content = format!(
        "name: detect-test\nadapter: {adapter}\nspecify_version: '{version}'\nplatforms:\n{platforms}",
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

fn scaffold_core(root: &Path) {
    let dir = root.join("shared/src");
    fs::create_dir_all(&dir).expect("mkdir shared/src");
    fs::write(dir.join("app.rs"), "pub struct App;").expect("write app.rs");
}

fn scaffold_ios(root: &Path) {
    let dir = root.join("iOS/TestApp");
    fs::create_dir_all(&dir).expect("mkdir iOS/TestApp");
    fs::write(dir.join("ContentView.swift"), "struct ContentView {}").expect("write swift");
}

fn scaffold_android(root: &Path) {
    let dir = root.join("Android/app/src/main/kotlin/com/test");
    fs::create_dir_all(&dir).expect("mkdir Android");
    fs::write(dir.join("MainActivity.kt"), "class MainActivity").expect("write kt");
}

fn detect_fixture(platforms: &[&str]) -> tempfile::TempDir {
    let tmp = tempdir().expect("tempdir");
    write_project_yaml(tmp.path(), "vectis", platforms);
    seed_vectis_adapter(tmp.path());
    tmp
}

#[test]
fn greenfield_reports_all_supported_missing() {
    let fixture = detect_fixture(&["core", "ios", "android"]);
    let missing = vectis_missing_platforms(fixture.path(), &ALL_SUPPORTED).expect("detect ok");
    assert_eq!(missing, ALL_SUPPORTED.to_vec());
}

#[test]
fn partial_shells_missing_android() {
    let fixture = detect_fixture(&["core", "ios", "android"]);
    scaffold_core(fixture.path());
    scaffold_ios(fixture.path());

    let missing = vectis_missing_platforms(fixture.path(), &ALL_SUPPORTED).expect("detect ok");
    assert_eq!(missing, vec![Platform::Android]);
}

#[test]
fn all_shells_present_returns_empty() {
    let fixture = detect_fixture(&["core", "ios", "android"]);
    scaffold_core(fixture.path());
    scaffold_ios(fixture.path());
    scaffold_android(fixture.path());

    let missing = vectis_missing_platforms(fixture.path(), &ALL_SUPPORTED).expect("detect ok");
    assert!(missing.is_empty());
}

#[test]
fn non_vectis_skips_detect() {
    let tmp = tempdir().expect("tempdir");
    write_project_yaml(tmp.path(), "omnia", &["core", "ios"]);

    let adapter = tmp.path().join("adapters/targets/omnia");
    let briefs = adapter.join("briefs");
    fs::create_dir_all(&briefs).expect("mkdir briefs");
    fs::write(
        adapter.join("adapter.yaml"),
        "name: omnia\nversion: 1.0.0\naxis: target\nexecution: agent\nbriefs:\n  shape: briefs/shape.md\n  build: briefs/build.md\n  merge: briefs/merge.md\ndescription: Test omnia adapter\n",
    )
    .expect("write adapter.yaml");
    for op in ["shape", "build", "merge"] {
        fs::write(
            briefs.join(format!("{op}.md")),
            format!("---\nid: {op}\ndescription: {op} brief\n---\n"),
        )
        .expect("write brief");
    }

    let missing = vectis_missing_platforms(tmp.path(), &ALL_SUPPORTED).expect("non-vectis ok");
    assert!(missing.is_empty(), "non-vectis projects must not invoke detect");
}

#[test]
fn empty_declared_skips_dispatch() {
    let fixture = detect_fixture(&["core", "ios", "android"]);
    let missing = vectis_missing_platforms(fixture.path(), &[]).expect("empty declared ok");
    assert!(missing.is_empty());
}
