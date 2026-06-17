use std::collections::BTreeSet;
use std::fs;

use tempfile::tempdir;

use super::{
    EffectiveAssets, MaterializeScope, resolve_effective_assets, resolve_materialize_scope,
};
use crate::Platform;
use crate::platform::bootstrap_context_from_missing;

mod needs_materialize;

fn write_assets_yaml(dir: &std::path::Path, yaml: &str) {
    fs::write(dir.join("assets.yaml"), yaml).expect("write assets.yaml");
}

fn design_system_bulk_fixture() -> (tempfile::TempDir, EffectiveAssets) {
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
         \x20\x20\x20\x20source: assets/settings.svg\n\
         \x20\x20pinned:\n\
         \x20\x20\x20\x20kind: vector\n\
         \x20\x20\x20\x20role: icon\n\
         \x20\x20\x20\x20source: assets/pinned.svg\n\
         \x20\x20\x20\x20sources:\n\
         \x20\x20\x20\x20\x20\x20ios: assets/exports/ios/pinned.imageset/pinned.pdf\n\
         \x20\x20\x20\x20\x20\x20android: assets/exports/android/drawable/pinned.xml\n\
         \x20\x20symbol-only:\n\
         \x20\x20\x20\x20kind: symbol\n\
         \x20\x20\x20\x20role: icon\n\
         \x20\x20\x20\x20symbols:\n\
         \x20\x20\x20\x20\x20\x20ios: gear\n\
         \x20\x20\x20\x20\x20\x20android: Settings\n",
    );
    fs::write(slice.join("assets/settings.svg"), "<svg/>").expect("write svg");
    fs::write(slice.join("assets/pinned.svg"), "<svg/>").expect("write pinned svg");
    let exports = slice.join("assets/exports");
    fs::create_dir_all(exports.join("ios/pinned.imageset")).expect("mkdir ios export");
    fs::write(exports.join("ios/pinned.imageset/pinned.pdf"), "%PDF").expect("write pdf");
    fs::create_dir_all(exports.join("android/drawable")).expect("mkdir android export");
    fs::write(exports.join("android/drawable/pinned.xml"), "<vector/>").expect("write xml");

    let effective = resolve_effective_assets(&slice, tmp.path()).expect("effective assets");
    assert!(effective.slice_local);
    (tmp, effective)
}

mod bulk {
    use super::*;

    /// Slice-local inventory bulk pass: unpinned `source:` entries only.
    #[test]
    fn unpinned_source_only() {
        let (tmp, effective) = design_system_bulk_fixture();
        let slice = tmp.path().join(".specify/slices/design-system");
        let bootstrap = bootstrap_context_from_missing(&[]);

        let scope = resolve_materialize_scope(&slice, tmp.path(), &bootstrap, &effective);

        assert_eq!(scope.asset_ids, BTreeSet::from([String::from("settings")]));
    }
}

mod feature {
    use super::*;

    /// Incremental pass: composition refs win over design.md mentions.
    #[test]
    fn composition_precedence() {
        let tmp = tempdir().expect("tempdir");
        let slice = tmp.path().join(".specify/slices/task-list");
        fs::create_dir_all(slice.join("specs/tasks")).expect("mkdir specs");
        let design = tmp.path().join("design-system");
        fs::create_dir_all(design.join("assets")).expect("mkdir assets");
        fs::write(
            design.join("assets.yaml"),
            "version: 1\n\
             assets:\n\
             \x20\x20settings:\n\
             \x20\x20\x20\x20kind: vector\n\
             \x20\x20\x20\x20role: icon\n\
             \x20\x20\x20\x20source: assets/settings.svg\n\
             \x20\x20hero:\n\
             \x20\x20\x20\x20kind: raster\n\
             \x20\x20\x20\x20role: illustration\n\
             \x20\x20\x20\x20source: assets/hero.png\n\
             \x20\x20plus:\n\
             \x20\x20\x20\x20kind: symbol\n\
             \x20\x20\x20\x20role: icon\n",
        )
        .expect("write project assets");
        fs::write(
            slice.join("composition.yaml"),
            "screens:\n\
             \x20\x20home:\n\
             \x20\x20\x20\x20items:\n\
             \x20\x20\x20\x20\x20\x20- icon:\n\
             \x20\x20\x20\x20\x20\x20\x20\x20\x20\x20name: settings\n",
        )
        .expect("write composition");
        fs::write(slice.join("design.md"), "The empty state SHALL render the `hero` illustration.")
            .expect("write design");

        let effective = resolve_effective_assets(&slice, tmp.path()).expect("effective assets");
        assert!(!effective.slice_local);

        let scope = resolve_materialize_scope(
            &slice,
            tmp.path(),
            &bootstrap_context_from_missing(&[]),
            &effective,
        );

        assert_eq!(scope.asset_ids, BTreeSet::from([String::from("settings")]));
    }

    /// Without composition.yaml, spec.md and design.md supply refs.
    #[test]
    fn artifact_refs() {
        let tmp = tempdir().expect("tempdir");
        let slice = tmp.path().join(".specify/slices/task-list");
        fs::create_dir_all(slice.join("specs/tasks")).expect("mkdir specs");
        let design = tmp.path().join("design-system");
        fs::create_dir_all(&design).expect("mkdir design-system");
        fs::write(
            design.join("assets.yaml"),
            "version: 1\n\
             assets:\n\
             \x20\x20empty-tasks-hero:\n\
             \x20\x20\x20\x20kind: raster\n\
             \x20\x20\x20\x20role: illustration\n\
             \x20\x20\x20\x20source: assets/empty-tasks-hero.png\n\
             \x20\x20settings:\n\
             \x20\x20\x20\x20kind: vector\n\
             \x20\x20\x20\x20role: icon\n",
        )
        .expect("write assets");
        fs::write(
            slice.join("design.md"),
            "The asset `empty-tasks-hero` referenced from REQ-001's empty-state scenario must exist.",
        )
        .expect("write design");
        fs::write(
            slice.join("specs/tasks/spec.md"),
            "SF Symbols (`settings`) resolve at call sites without inventory copy.",
        )
        .expect("write spec");

        let effective = resolve_effective_assets(&slice, tmp.path()).expect("effective assets");
        let scope = resolve_materialize_scope(
            &slice,
            tmp.path(),
            &bootstrap_context_from_missing(&[]),
            &effective,
        );

        let expected = MaterializeScope {
            asset_ids: BTreeSet::from([String::from("empty-tasks-hero"), String::from("settings")]),
        };
        assert_eq!(scope, expected);
    }
}

mod bootstrap {
    use super::*;

    /// §6.1 ∧ ¬§6.2: app-icon enters scope when unsatisfied.
    #[test]
    fn unsatisfied_6_2() {
        let tmp = tempdir().expect("tempdir");
        let slice = tmp.path().join(".specify/slices/app-foundation");
        fs::create_dir_all(slice.join("specs/core")).expect("mkdir specs");
        let design = tmp.path().join("design-system");
        fs::create_dir_all(design.join("assets")).expect("mkdir assets");
        fs::write(
            design.join("assets.yaml"),
            "version: 1\n\
             app-icon: app-icon\n\
             assets:\n\
             \x20\x20app-icon:\n\
             \x20\x20\x20\x20kind: vector\n\
             \x20\x20\x20\x20role: app-icon\n",
        )
        .expect("write assets");
        fs::write(slice.join("design.md"), "Bootstrap shell scaffold.").expect("write design");

        let effective = resolve_effective_assets(&slice, tmp.path()).expect("effective assets");
        let bootstrap = bootstrap_context_from_missing(&[Platform::Ios, Platform::Android]);

        let scope = resolve_materialize_scope(&slice, tmp.path(), &bootstrap, &effective);

        assert!(scope.asset_ids.contains("app-icon"));
    }

    /// Path A master on disk satisfies §6.2 — app-icon omitted.
    #[test]
    fn satisfied_6_2_omitted() {
        let tmp = tempdir().expect("tempdir");
        let slice = tmp.path().join(".specify/slices/app-foundation");
        fs::create_dir_all(slice.join("specs/core")).expect("mkdir specs");
        let design = tmp.path().join("design-system");
        fs::create_dir_all(design.join("assets")).expect("mkdir assets");
        fs::write(
            design.join("assets.yaml"),
            "version: 1\n\
             app-icon: app-icon\n\
             assets:\n\
             \x20\x20app-icon:\n\
             \x20\x20\x20\x20kind: vector\n\
             \x20\x20\x20\x20role: app-icon\n\
             \x20\x20\x20\x20source: assets/app-icon.svg\n",
        )
        .expect("write assets");
        fs::write(design.join("assets/app-icon.svg"), "<svg/>").expect("write svg");
        fs::write(slice.join("design.md"), "Bootstrap shell scaffold.").expect("write design");

        let effective = resolve_effective_assets(&slice, tmp.path()).expect("effective assets");
        let bootstrap = bootstrap_context_from_missing(&[Platform::Ios, Platform::Android]);

        let scope = resolve_materialize_scope(&slice, tmp.path(), &bootstrap, &effective);

        assert!(!scope.asset_ids.contains("app-icon"));
    }
}

mod effective_path {
    use super::*;

    #[test]
    fn slice_local_wins() {
        let tmp = tempdir().expect("tempdir");
        let slice = tmp.path().join(".specify/slices/design-system");
        fs::create_dir_all(&slice).expect("mkdir slice");
        fs::create_dir_all(tmp.path().join("design-system")).expect("mkdir project ds");
        write_assets_yaml(&slice, "version: 1\nassets: {}\n");
        write_assets_yaml(&tmp.path().join("design-system"), "version: 1\nassets: {}\n");

        let effective = resolve_effective_assets(&slice, tmp.path()).expect("effective");
        assert!(effective.slice_local);
        assert_eq!(effective.path, slice.join("assets.yaml"));
    }
}
