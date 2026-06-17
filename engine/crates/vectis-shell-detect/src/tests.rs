//! Unit tests for Crux shell presence heuristics.

use std::path::Path;

use tempfile::tempdir;

use super::{SUPPORTED_SHELL_PLATFORMS, missing_shell_platforms, shell_present};

fn scaffold_core(root: &Path) {
    let dir = root.join("shared/src");
    std::fs::create_dir_all(&dir).expect("mkdir shared/src");
    std::fs::write(dir.join("app.rs"), "pub struct App;").expect("write app.rs");
}

fn scaffold_ios(root: &Path) {
    let dir = root.join("iOS/TestApp");
    std::fs::create_dir_all(&dir).expect("mkdir iOS/TestApp");
    std::fs::write(dir.join("ContentView.swift"), "struct ContentView {}").expect("write swift");
}

fn scaffold_android(root: &Path) {
    let dir = root.join("Android/app/src/main/kotlin/com/test");
    std::fs::create_dir_all(&dir).expect("mkdir Android");
    std::fs::write(dir.join("MainActivity.kt"), "class MainActivity").expect("write kt");
}

#[test]
fn all_present_empty_missing() {
    let tmp = tempdir().unwrap();
    scaffold_core(tmp.path());
    scaffold_ios(tmp.path());
    scaffold_android(tmp.path());

    let missing = missing_shell_platforms(tmp.path(), &["core", "ios", "android"]);
    assert!(missing.is_empty(), "expected empty missing set: {missing:?}");
}

#[test]
fn missing_ios_only() {
    let tmp = tempdir().unwrap();
    scaffold_core(tmp.path());
    scaffold_android(tmp.path());

    let missing = missing_shell_platforms(tmp.path(), &["core", "ios", "android"]);
    assert_eq!(missing, vec!["ios".to_string()]);
}

#[test]
fn greenfield_all_supported_missing() {
    let tmp = tempdir().unwrap();

    let missing = missing_shell_platforms(tmp.path(), &["core", "ios", "android"]);
    assert_eq!(missing.len(), 3);
    assert!(missing.iter().any(|p| p == "core"));
    assert!(missing.iter().any(|p| p == "ios"));
    assert!(missing.iter().any(|p| p == "android"));
}

#[test]
fn web_desktop_not_in_missing() {
    let tmp = tempdir().unwrap();
    scaffold_core(tmp.path());

    let missing = missing_shell_platforms(tmp.path(), &["core", "web", "desktop"]);
    assert!(missing.is_empty(), "web/desktop should not appear in missing: {missing:?}");
    assert!(shell_present(tmp.path(), "web"));
    assert!(shell_present(tmp.path(), "desktop"));
}

#[test]
fn ios_without_swift_not_present() {
    let tmp = tempdir().unwrap();
    scaffold_core(tmp.path());
    let ios_dir = tmp.path().join("iOS");
    std::fs::create_dir_all(&ios_dir).expect("mkdir iOS");
    std::fs::write(ios_dir.join("README.md"), "placeholder").expect("write readme");

    assert!(!shell_present(tmp.path(), "ios"));
    let missing = missing_shell_platforms(tmp.path(), &["core", "ios"]);
    assert!(missing.iter().any(|p| p == "ios"));
}

#[test]
fn android_without_kt_not_present() {
    let tmp = tempdir().unwrap();
    scaffold_core(tmp.path());
    let android_dir = tmp.path().join("Android");
    std::fs::create_dir_all(&android_dir).expect("mkdir Android");
    std::fs::write(android_dir.join("build.gradle"), "").expect("write gradle");

    assert!(!shell_present(tmp.path(), "android"));
    let missing = missing_shell_platforms(tmp.path(), &["core", "android"]);
    assert!(missing.iter().any(|p| p == "android"));
}

#[test]
fn supported_platforms_closed_set() {
    assert_eq!(SUPPORTED_SHELL_PLATFORMS, &["core", "ios", "android"]);
}
