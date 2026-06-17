use std::collections::BTreeSet;
use std::fs;

use tempfile::tempdir;

use super::super::{EffectiveAssets, MaterializeScope, scope_needs_materialize};
use crate::Platform;

fn write_assets_yaml(dir: &std::path::Path, yaml: &str) {
    fs::write(dir.join("assets.yaml"), yaml).expect("write assets.yaml");
}

#[test]
fn exports_present() {
    let tmp = tempdir().expect("tempdir");
    let slice = tmp.path().join(".specify/slices/design-system");
    fs::create_dir_all(slice.join("assets")).expect("mkdir assets");
    write_assets_yaml(
        &slice,
        "version: 1\n\
         assets:\n\
         \x20\x20settings:\n\
         \x20\x20\x20\x20kind: vector\n\
         \x20\x20\x20\x20role: icon\n\
         \x20\x20\x20\x20source: assets/settings.svg\n",
    );
    fs::write(slice.join("assets/settings.svg"), "<svg/>").expect("write svg");
    let exports = slice.join("assets/exports");
    fs::create_dir_all(exports.join("ios/settings.imageset")).expect("mkdir ios export");
    fs::write(exports.join("ios/settings.imageset/settings.pdf"), "%PDF").expect("write pdf");
    fs::create_dir_all(exports.join("android/drawable")).expect("mkdir android export");
    fs::write(exports.join("android/drawable/settings.xml"), "<vector/>").expect("write xml");

    let effective = EffectiveAssets {
        path: slice.join("assets.yaml"),
        slice_local: true,
    };
    let scope = MaterializeScope {
        asset_ids: BTreeSet::from([String::from("settings")]),
    };

    assert!(!scope_needs_materialize(&scope, &effective, &[Platform::Ios, Platform::Android]));
}

#[test]
fn ios_export_missing() {
    let tmp = tempdir().expect("tempdir");
    let slice = tmp.path().join(".specify/slices/design-system");
    fs::create_dir_all(slice.join("assets")).expect("mkdir assets");
    write_assets_yaml(
        &slice,
        "version: 1\n\
         assets:\n\
         \x20\x20settings:\n\
         \x20\x20\x20\x20kind: vector\n\
         \x20\x20\x20\x20role: icon\n\
         \x20\x20\x20\x20source: assets/settings.svg\n",
    );
    fs::write(slice.join("assets/settings.svg"), "<svg/>").expect("write svg");

    let effective = EffectiveAssets {
        path: slice.join("assets.yaml"),
        slice_local: true,
    };
    let scope = MaterializeScope {
        asset_ids: BTreeSet::from([String::from("settings")]),
    };

    assert!(scope_needs_materialize(&scope, &effective, &[Platform::Ios, Platform::Android]));
}

#[test]
fn empty_scope() {
    let tmp = tempdir().expect("tempdir");
    let slice = tmp.path().join(".specify/slices/feature");
    fs::create_dir_all(&slice).expect("mkdir slice");
    write_assets_yaml(&slice, "version: 1\nassets: {}\n");

    let effective = EffectiveAssets {
        path: slice.join("assets.yaml"),
        slice_local: true,
    };

    assert!(!scope_needs_materialize(
        &MaterializeScope::default(),
        &effective,
        &[Platform::Ios, Platform::Android]
    ));
}
