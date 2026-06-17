//! Unit tests for shell-resident launcher icon probes.

use std::path::Path;

use tempfile::tempdir;

use crate::shell_resident_app_icon;

fn scaffold_ios_appiconset(root: &Path, contents_json: &str, png_bytes: Option<&[u8]>) {
    let appiconset = root.join("iOS/TestApp/Resources/Assets.xcassets/AppIcon.appiconset");
    std::fs::create_dir_all(&appiconset).expect("mkdir appiconset");
    std::fs::write(appiconset.join("Contents.json"), contents_json).expect("write Contents.json");
    if let Some(bytes) = png_bytes {
        std::fs::write(appiconset.join("AppIcon.png"), bytes).expect("write png");
    }
}

fn minimal_png() -> Vec<u8> {
    // 1×1 RGBA PNG — sufficient for presence probe (not dimension validation).
    vec![
        0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f,
        0x15, 0xc4, 0x89, 0x00, 0x00, 0x00, 0x0a, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9c, 0x63, 0x00,
        0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0d, 0x0a, 0x2d, 0xb4, 0x00, 0x00, 0x00, 0x00, 0x49,
        0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
    ]
}

#[test]
fn ios_skeleton_without_png_false() {
    let tmp = tempdir().unwrap();
    let contents = r#"{
  "images": [{ "filename": "AppIcon.png", "idiom": "universal", "platform": "ios", "size": "1024x1024" }],
  "info": { "author": "xcode", "version": 1 }
}"#;
    scaffold_ios_appiconset(tmp.path(), contents, None);

    assert!(!shell_resident_app_icon(tmp.path(), "ios"));
}

#[test]
fn ios_referenced_png_true() {
    let tmp = tempdir().unwrap();
    let contents = r#"{
  "images": [{ "filename": "AppIcon.png", "idiom": "universal", "platform": "ios", "size": "1024x1024" }],
  "info": { "author": "xcode", "version": 1 }
}"#;
    scaffold_ios_appiconset(tmp.path(), contents, Some(&minimal_png()));

    assert!(shell_resident_app_icon(tmp.path(), "ios"));
}

#[test]
fn ios_contents_without_images_false() {
    let tmp = tempdir().unwrap();
    scaffold_ios_appiconset(tmp.path(), r#"{"info":{"version":1}}"#, Some(&minimal_png()));

    assert!(!shell_resident_app_icon(tmp.path(), "ios"));
}

#[test]
fn android_adaptive_xml_true() {
    let tmp = tempdir().unwrap();
    let dir = tmp.path().join("Android/app/src/main/res/mipmap-anydpi-v26");
    std::fs::create_dir_all(&dir).expect("mkdir mipmap-anydpi-v26");
    std::fs::write(dir.join("ic_launcher.xml"), "<adaptive-icon/>").expect("write xml");

    assert!(shell_resident_app_icon(tmp.path(), "android"));
}

#[test]
fn android_legacy_mipmap_true() {
    let tmp = tempdir().unwrap();
    let dir = tmp.path().join("Android/app/src/main/res/mipmap-mdpi");
    std::fs::create_dir_all(&dir).expect("mkdir mipmap-mdpi");
    std::fs::write(dir.join("ic_launcher.png"), minimal_png()).expect("write png");

    assert!(shell_resident_app_icon(tmp.path(), "android"));
}

#[test]
fn android_no_launcher_false() {
    let tmp = tempdir().unwrap();
    let dir = tmp.path().join("Android/app/src/main/res/values");
    std::fs::create_dir_all(&dir).expect("mkdir values");
    std::fs::write(dir.join("strings.xml"), "<resources/>").expect("write strings");

    assert!(!shell_resident_app_icon(tmp.path(), "android"));
}

#[test]
fn core_platform_false() {
    let tmp = tempdir().unwrap();
    assert!(!shell_resident_app_icon(tmp.path(), "core"));
}
