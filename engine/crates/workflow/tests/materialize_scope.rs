//! Filesystem tests for `specify_workflow::slice::build::materialize_scope`.
//!
//! RFC §2.1 in-scope asset resolution. Each test seeds a throw-away slice +
//! project tree under `tempfile::TempDir` and drives the public resolver
//! surface (`resolve_effective_assets`, `resolve_materialize_scope`,
//! `scope_needs_materialize`, `materialize_platform_csv`) across the
//! composition / artifact-text / unpinned-source reference paths and the
//! per-platform export-satisfaction checks (Apple imageset / app-icon set,
//! Android drawable / mipmap).

use std::fs;
use std::path::{Path, PathBuf};

use specify_workflow::Platform;
use specify_workflow::slice::build::materialize_scope::{
    EffectiveAssets, MaterializeScope, materialize_platform_csv, resolve_effective_assets,
    resolve_materialize_scope, scope_needs_materialize,
};
use tempfile::TempDir;

fn write(path: &Path, body: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent dir");
    }
    fs::write(path, body).expect("write fixture file");
}

/// Throw-away `project` + slice tree. `design-system/assets.yaml` is the
/// project inventory; `${slice}/assets.yaml` is the slice-local override.
struct Fixture {
    _tmp: TempDir,
    project: PathBuf,
    slice: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let tmp = TempDir::new().expect("tempdir");
        let project = tmp.path().join("project");
        let slice = project.join(".specify/slices/feature");
        fs::create_dir_all(&slice).expect("slice dir");
        Self {
            _tmp: tmp,
            project,
            slice,
        }
    }

    fn project_assets(&self, body: &str) {
        write(&self.project.join("design-system/assets.yaml"), body);
    }

    fn slice_assets(&self, body: &str) {
        write(&self.slice.join("assets.yaml"), body);
    }

    fn slice_file(&self, rel: &str, body: &str) {
        write(&self.slice.join(rel), body);
    }

    /// Seed a file relative to the project inventory's parent
    /// (`design-system/`), where project-level exports live.
    fn project_export(&self, rel: &str) {
        write(&self.project.join("design-system").join(rel), "x");
    }

    fn effective(&self) -> EffectiveAssets {
        resolve_effective_assets(&self.slice, &self.project).expect("effective assets resolved")
    }
}

#[test]
fn effective_prefers_slice_local() {
    let f = Fixture::new();
    f.project_assets("assets: {}\n");
    f.slice_assets("assets: {}\n");

    let eff = resolve_effective_assets(&f.slice, &f.project).expect("resolved");
    assert!(eff.slice_local, "slice-local inventory wins");
    assert_eq!(eff.path, f.slice.join("assets.yaml"));
}

#[test]
fn effective_falls_back_to_project() {
    let f = Fixture::new();
    f.project_assets("assets: {}\n");

    let eff = resolve_effective_assets(&f.slice, &f.project).expect("resolved");
    assert!(!eff.slice_local, "project inventory used when no slice-local file");
    assert_eq!(eff.path, f.project.join("design-system/assets.yaml"));
}

#[test]
fn effective_none_when_absent() {
    let f = Fixture::new();
    assert!(resolve_effective_assets(&f.slice, &f.project).is_none());
}

#[test]
fn platform_csv_joins_kebab() {
    assert_eq!(materialize_platform_csv(&[Platform::Ios, Platform::Android]), "ios,android");
    assert_eq!(materialize_platform_csv(&[Platform::Ios]), "ios");
    assert_eq!(materialize_platform_csv(&[]), "");
}

#[test]
fn scope_composition_materializable_only() {
    let f = Fixture::new();
    f.project_assets(
        "assets:\n  hero-logo:\n    kind: vector\n  menu-icon:\n    kind: raster\n  blurb:\n    kind: text\n",
    );
    f.slice_file(
        "composition.yaml",
        "screens:\n  - image:\n      name: hero-logo\n  - icon-button:\n      icon: menu-icon\n  - icon:\n      name: blurb\n",
    );

    let scope = resolve_materialize_scope(&f.slice, &f.project, &[], &f.effective());
    assert!(scope.asset_ids.contains("hero-logo"), "image.name vector kept");
    assert!(scope.asset_ids.contains("menu-icon"), "icon-button.icon raster kept");
    assert!(!scope.asset_ids.contains("blurb"), "text kind filtered out");
}

#[test]
fn scope_artifact_text_refs() {
    let f = Fixture::new();
    f.project_assets(
        "assets:\n  brand-mark:\n    kind: vector\n  logo:\n    kind: vector\n  unused-glyph:\n    kind: vector\n",
    );
    // Backtick reference in design.md; dotted `assets.<id>` reference in a spec.
    f.slice_file("design.md", "The header shows the `brand-mark` prominently.\n");
    f.slice_file("specs/auth/spec.md", "Bind assets.logo to the login screen.\n");

    let scope = resolve_materialize_scope(&f.slice, &f.project, &[], &f.effective());
    assert!(scope.asset_ids.contains("brand-mark"), "backtick reference matched");
    assert!(scope.asset_ids.contains("logo"), "dotted assets.<id> reference matched");
    assert!(!scope.asset_ids.contains("unused-glyph"), "unreferenced asset excluded");
}

#[test]
fn scope_app_icon_with_ui_platform() {
    let f = Fixture::new();
    f.project_assets(
        "app-icon: launcher\nassets:\n  launcher:\n    role: app-icon\n    source: src/icon.svg\n",
    );
    f.slice_file("design.md", "no asset references here\n");
    let eff = f.effective();

    let with_ui = resolve_materialize_scope(&f.slice, &f.project, &[Platform::Ios], &eff);
    assert!(with_ui.asset_ids.contains("launcher"), "app-icon appended for UI platform");

    let no_ui = resolve_materialize_scope(&f.slice, &f.project, &[], &eff);
    assert!(!no_ui.asset_ids.contains("launcher"), "app-icon skipped without UI platforms");
}

#[test]
fn scope_unpinned_source_slice_local() {
    let f = Fixture::new();
    // Slice-local inventory; no composition / artifact text => the only path
    // into scope is the unpinned-source sweep (slice-local only).
    f.slice_assets("assets:\n  brand-mark:\n    kind: vector\n    source: src/brand.svg\n");

    let eff = f.effective();
    assert!(eff.slice_local);
    let scope = resolve_materialize_scope(&f.slice, &f.project, &[Platform::Ios], &eff);
    assert!(scope.asset_ids.contains("brand-mark"), "unpinned source asset swept in");
}

#[test]
fn scope_excludes_pinned_source() {
    let f = Fixture::new();
    f.slice_assets(
        "assets:\n  brand-mark:\n    kind: vector\n    source: src/brand.svg\n    sources:\n      ios: exports/ios/brand.svg\n",
    );
    f.slice_file("exports/ios/brand.svg", "<svg/>\n");

    let scope = resolve_materialize_scope(&f.slice, &f.project, &[Platform::Ios], &f.effective());
    assert!(!scope.asset_ids.contains("brand-mark"), "pinned-and-present asset is satisfied");
}

#[test]
fn needs_false_for_empty_scope() {
    let f = Fixture::new();
    f.project_assets("assets: {}\n");
    assert!(!scope_needs_materialize(
        &MaterializeScope::default(),
        &f.effective(),
        &[Platform::Ios],
    ));
}

#[test]
fn needs_true_app_icon_no_export() {
    let f = Fixture::new();
    f.project_assets(
        "app-icon: launcher\nassets:\n  launcher:\n    role: app-icon\n    source: src/icon.svg\n",
    );
    let eff = f.effective();
    let scope = resolve_materialize_scope(&f.slice, &f.project, &[Platform::Ios], &eff);

    assert!(scope.asset_ids.contains("launcher"));
    assert!(scope_needs_materialize(&scope, &eff, &[Platform::Ios]), "no iOS app-icon export");
}

#[test]
fn needs_false_ios_app_icon_export() {
    let f = Fixture::new();
    f.project_assets(
        "app-icon: launcher\nassets:\n  launcher:\n    role: app-icon\n    source: src/icon.svg\n",
    );
    f.project_export("assets/exports/ios/app-icon/AppIcon.appiconset/Contents.json");
    f.project_export("assets/exports/ios/app-icon/AppIcon.appiconset/icon-60@2x.png");

    let eff = f.effective();
    let scope = resolve_materialize_scope(&f.slice, &f.project, &[Platform::Ios], &eff);
    assert!(!scope_needs_materialize(&scope, &eff, &[Platform::Ios]), "appiconset satisfies");
}

#[test]
fn needs_true_vector_no_export() {
    let f = Fixture::new();
    f.slice_assets("assets:\n  glyph:\n    kind: vector\n    source: src/glyph.svg\n");
    let eff = f.effective();
    let scope = resolve_materialize_scope(&f.slice, &f.project, &[Platform::Ios], &eff);

    assert!(scope.asset_ids.contains("glyph"));
    assert!(scope_needs_materialize(&scope, &eff, &[Platform::Ios]));
}

#[test]
fn needs_false_ios_imageset() {
    let f = Fixture::new();
    f.slice_assets("assets:\n  glyph:\n    kind: vector\n    source: src/glyph.svg\n");
    f.slice_file("assets/exports/ios/glyph.imageset/glyph.pdf", "x");

    let eff = f.effective();
    let scope = resolve_materialize_scope(&f.slice, &f.project, &[Platform::Ios], &eff);
    assert!(!scope_needs_materialize(&scope, &eff, &[Platform::Ios]), "imageset satisfies iOS");
}

#[test]
fn needs_false_android_vector() {
    let f = Fixture::new();
    f.slice_assets("assets:\n  nav-icon:\n    kind: vector\n    source: src/nav.svg\n");
    // kebab id maps to snake_case drawable filename.
    f.slice_file("assets/exports/android/drawable/nav_icon.xml", "<vector/>\n");

    let eff = f.effective();
    let scope = resolve_materialize_scope(&f.slice, &f.project, &[Platform::Android], &eff);
    assert!(!scope_needs_materialize(&scope, &eff, &[Platform::Android]), "drawable xml satisfies");
}

#[test]
fn needs_false_android_raster() {
    let f = Fixture::new();
    f.slice_assets("assets:\n  splash:\n    kind: raster\n    source: src/splash.png\n");
    f.slice_file("assets/exports/android/mipmap-xxhdpi/splash.png", "x");

    let eff = f.effective();
    let scope = resolve_materialize_scope(&f.slice, &f.project, &[Platform::Android], &eff);
    assert!(!scope_needs_materialize(&scope, &eff, &[Platform::Android]), "mipmap png satisfies");
}
