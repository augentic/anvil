//! Unit tests for [`super::bootstrap_context`] and
//! [`super::bootstrap_context_from_missing`].

use std::fs;
use std::path::Path;

use tempfile::tempdir;

use super::{bootstrap_context, bootstrap_context_from_missing};
use crate::Platform;

const ALL_SUPPORTED: [Platform; 3] = [Platform::Core, Platform::Ios, Platform::Android];

fn write_project_yaml(root: &Path, adapter: &str, platforms: &[&str]) {
    let yaml_platforms: Vec<String> = platforms.iter().map(|p| format!("  - {p}")).collect();
    let content = format!(
        "name: bootstrap-test\nadapter: {adapter}\nspecify_version: '{version}'\nplatforms:\n{platforms}",
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

fn bootstrap_fixture(platforms: &[&str]) -> tempfile::TempDir {
    let tmp = tempdir().expect("tempdir");
    write_project_yaml(tmp.path(), "vectis", platforms);
    seed_vectis_adapter(tmp.path());
    tmp
}

#[test]
fn from_missing_greenfield_filters_core() {
    let ctx = bootstrap_context_from_missing(&ALL_SUPPORTED);
    assert!(ctx.triggers);
    assert_eq!(ctx.missing_ui, vec![Platform::Ios, Platform::Android]);
}

#[test]
fn core_only_absent_no_trigger() {
    let ctx = bootstrap_context_from_missing(&[Platform::Core]);
    assert!(!ctx.triggers);
    assert!(ctx.missing_ui.is_empty());
}

#[test]
fn from_missing_single_ui_platform() {
    let ctx = bootstrap_context_from_missing(&[Platform::Android]);
    assert!(ctx.triggers);
    assert_eq!(ctx.missing_ui, vec![Platform::Android]);
}

#[test]
fn from_missing_empty_is_inert() {
    let ctx = bootstrap_context_from_missing(&[]);
    assert!(!ctx.triggers);
    assert!(ctx.missing_ui.is_empty());
}

#[test]
fn greenfield_triggers_ui() {
    let fixture = bootstrap_fixture(&["core", "ios", "android"]);
    let ctx = bootstrap_context(fixture.path()).expect("bootstrap ok");
    assert!(ctx.triggers);
    assert_eq!(ctx.missing_ui, vec![Platform::Ios, Platform::Android]);
}

#[test]
fn core_only_missing_does_not_trigger() {
    let fixture = bootstrap_fixture(&["core", "ios", "android"]);
    scaffold_ios(fixture.path());
    scaffold_android(fixture.path());

    let ctx = bootstrap_context(fixture.path()).expect("bootstrap ok");
    assert!(!ctx.triggers);
    assert!(ctx.missing_ui.is_empty());
}

#[test]
fn all_shells_present_does_not_trigger() {
    let fixture = bootstrap_fixture(&["core", "ios", "android"]);
    scaffold_core(fixture.path());
    scaffold_ios(fixture.path());
    scaffold_android(fixture.path());

    let ctx = bootstrap_context(fixture.path()).expect("bootstrap ok");
    assert!(!ctx.triggers);
    assert!(ctx.missing_ui.is_empty());
}

#[test]
fn non_vectis_does_not_trigger() {
    let tmp = tempdir().expect("tempdir");
    write_project_yaml(tmp.path(), "omnia", &["core", "ios", "android"]);

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

    let ctx = bootstrap_context(tmp.path()).expect("non-vectis ok");
    assert!(!ctx.triggers);
    assert!(ctx.missing_ui.is_empty());
}

#[test]
fn core_only_declared_never_triggers() {
    let fixture = bootstrap_fixture(&["core"]);
    let ctx = bootstrap_context(fixture.path()).expect("core-only ok");
    assert!(!ctx.triggers);
    assert!(ctx.missing_ui.is_empty());
}
