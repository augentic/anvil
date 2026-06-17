//! RFC §2.1 in-scope asset resolution for slice-build prepare.

#[cfg(test)]
#[path = "materialize_scope/tests.rs"]
mod tests;

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use regex::Regex;
use serde_json::Value;
use specify_vectis_shell_detect::shell_resident_app_icon;

use crate::Platform;
use crate::config::ProjectConfig;
use crate::platform::BootstrapContext;

const PROJECT_ASSETS_REL: &str = "design-system/assets.yaml";
const SLICE_ASSETS_NAME: &str = "assets.yaml";
const COMPOSITION_NAME: &str = "composition.yaml";

/// Resolved `assets.yaml` path and whether it lives in the slice tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveAssets {
    /// Absolute or project-relative path to the effective inventory file.
    pub path: PathBuf,
    /// `true` when [`Self::path`] is `${SLICE_DIR}/assets.yaml`.
    pub slice_local: bool,
}

/// Asset ids that `slice build --phase prepare` should consider for
/// materialization (RFC §2.1 reference set, filtered to materializable kinds).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MaterializeScope {
    /// Sorted, deduplicated asset ids in scope for the active slice.
    pub asset_ids: BTreeSet<String>,
}

/// Resolve the effective `assets.yaml` with slice-local → project precedence.
#[must_use]
pub fn resolve_effective_assets(slice_dir: &Path, project_dir: &Path) -> Option<EffectiveAssets> {
    let slice_local = slice_dir.join(SLICE_ASSETS_NAME);
    if slice_local.is_file() {
        return Some(EffectiveAssets {
            path: slice_local,
            slice_local: true,
        });
    }
    let project = project_dir.join(PROJECT_ASSETS_REL);
    if project.is_file() {
        return Some(EffectiveAssets {
            path: project,
            slice_local: false,
        });
    }
    None
}

/// Whether any in-scope asset lacks on-disk exports for a declared shell
/// platform (RFC §2.1 prepare trigger).
#[must_use]
pub fn scope_needs_materialize(
    scope: &MaterializeScope, effective: &EffectiveAssets, shell_platforms: &[Platform],
) -> bool {
    if scope.asset_ids.is_empty() {
        return false;
    }
    let Ok(raw) = fs::read_to_string(&effective.path) else {
        return false;
    };
    let Ok(doc) = serde_saphyr::from_str::<Value>(&raw) else {
        return false;
    };
    let Some(assets) = doc.get("assets").and_then(Value::as_object) else {
        return false;
    };
    let assets_dir = effective.path.parent().unwrap_or_else(|| Path::new("."));

    scope.asset_ids.iter().any(|id| {
        assets
            .get(id)
            .is_some_and(|entry| asset_needs_materialize(entry, id, assets_dir, shell_platforms))
    })
}

/// Comma-separated `--platform` tokens for declared UI shell platforms.
#[must_use]
pub fn materialize_platform_csv(shell_platforms: &[Platform]) -> String {
    shell_platforms.iter().map(ToString::to_string).collect::<Vec<_>>().join(",")
}

/// Derive the RFC §2.1 materialization reference set for a slice build.
///
/// Returns an empty scope when the effective inventory is absent or
/// unreadable. Does not check export presence — callers decide whether
/// to invoke materialize.
#[must_use]
pub fn resolve_materialize_scope(
    slice_dir: &Path, project_dir: &Path, bootstrap: &BootstrapContext, effective: &EffectiveAssets,
) -> MaterializeScope {
    let Ok(raw) = fs::read_to_string(&effective.path) else {
        return MaterializeScope::default();
    };
    let Ok(doc) = serde_saphyr::from_str::<Value>(&raw) else {
        return MaterializeScope::default();
    };
    let Some(assets) = doc.get("assets").and_then(Value::as_object) else {
        return MaterializeScope::default();
    };

    let assets_dir = effective.path.parent().unwrap_or(project_dir);
    let shell_platforms = shell_platforms(project_dir);

    let mut reference_ids = collect_reference_ids(slice_dir, assets);
    if effective.slice_local {
        reference_ids.extend(unpinned_source_inventory(assets, assets_dir, &shell_platforms));
    }

    let mut asset_ids: BTreeSet<String> = reference_ids
        .into_iter()
        .filter(|id| assets.get(id).is_some_and(is_materializable_kind))
        .collect();

    append_bootstrap_app_icon(&mut asset_ids, project_dir, assets_dir, bootstrap, &doc, assets);

    MaterializeScope { asset_ids }
}

fn shell_platforms(project_dir: &Path) -> Vec<Platform> {
    let Ok(config) = ProjectConfig::load(project_dir) else {
        return vec![Platform::Ios, Platform::Android];
    };
    config
        .platforms
        .iter()
        .copied()
        .filter(|p| matches!(p, Platform::Ios | Platform::Android))
        .collect()
}

fn collect_reference_ids(
    slice_dir: &Path, assets: &serde_json::Map<String, Value>,
) -> BTreeSet<String> {
    let composition = slice_dir.join(COMPOSITION_NAME);
    if composition.is_file() {
        return collect_composition_asset_refs(&composition);
    }
    collect_artifact_asset_refs(slice_dir, assets)
}

fn collect_composition_asset_refs(path: &Path) -> BTreeSet<String> {
    let Ok(text) = fs::read_to_string(path) else {
        return BTreeSet::new();
    };
    let Ok(doc) = serde_saphyr::from_str::<Value>(&text) else {
        return BTreeSet::new();
    };
    collect_composition_asset_refs_value(&doc)
}

fn collect_composition_asset_refs_value(value: &Value) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    walk_composition_node(value, &mut ids);
    ids
}

fn walk_composition_node(node: &Value, ids: &mut BTreeSet<String>) {
    match node {
        Value::Object(map) => {
            for (key, val) in map {
                match key.as_str() {
                    "image" | "icon" => {
                        if let Some(name) = val.get("name").and_then(Value::as_str) {
                            ids.insert(name.to_string());
                        }
                    }
                    "icon-button" | "fab" => {
                        if let Some(icon) = val.get("icon").and_then(Value::as_str) {
                            ids.insert(icon.to_string());
                        }
                    }
                    _ => {}
                }
                walk_composition_node(val, ids);
            }
        }
        Value::Array(arr) => {
            for v in arr {
                walk_composition_node(v, ids);
            }
        }
        _ => {}
    }
}

fn collect_artifact_asset_refs(
    slice_dir: &Path, assets: &serde_json::Map<String, Value>,
) -> BTreeSet<String> {
    let mut corpus = String::new();
    append_artifact_text(slice_dir.join("design.md"), &mut corpus);
    let specs_dir = slice_dir.join("specs");
    if specs_dir.is_dir()
        && let Ok(entries) = fs::read_dir(&specs_dir)
    {
        for entry in entries.flatten() {
            let domain = entry.path();
            if domain.is_dir() {
                append_artifact_text(domain.join("spec.md"), &mut corpus);
            }
        }
    }
    if corpus.is_empty() {
        return BTreeSet::new();
    }

    assets.keys().filter(|id| text_references_asset(&corpus, id)).cloned().collect()
}

fn append_artifact_text(path: PathBuf, corpus: &mut String) {
    if let Ok(text) = fs::read_to_string(path) {
        corpus.push_str(&text);
        corpus.push('\n');
    }
}

fn text_references_asset(text: &str, asset_id: &str) -> bool {
    if text.contains(&format!("`{asset_id}`")) {
        return true;
    }
    if text.contains(&format!("assets.{asset_id}")) {
        return true;
    }
    asset_id_word_re(asset_id).is_ok_and(|re| re.is_match(text))
}

fn asset_id_word_re(asset_id: &str) -> Result<Regex, regex::Error> {
    let escaped = regex::escape(asset_id);
    Regex::new(&format!(r"(?m)(?<![a-z0-9-]){escaped}(?![a-z0-9-])"))
}

fn unpinned_source_inventory(
    assets: &serde_json::Map<String, Value>, assets_dir: &Path, shell_platforms: &[Platform],
) -> BTreeSet<String> {
    assets
        .iter()
        .filter(|(_, entry)| entry.get("source").and_then(Value::as_str).is_some())
        .filter(|(_, entry)| entry_lacks_satisfiable_pin(entry, assets_dir, shell_platforms))
        .map(|(id, _)| id.clone())
        .collect()
}

fn entry_lacks_satisfiable_pin(
    entry: &Value, assets_dir: &Path, shell_platforms: &[Platform],
) -> bool {
    if entry.get("source").and_then(Value::as_str).is_none() {
        return false;
    }
    shell_platforms
        .iter()
        .any(|platform| !platform_pin_active(entry, &platform.to_string(), assets_dir))
}

fn platform_pin_active(entry: &Value, platform: &str, assets_dir: &Path) -> bool {
    let Some(pin) = entry.get("sources").and_then(|s| s.get(platform)).and_then(Value::as_str)
    else {
        return false;
    };
    assets_dir.join(pin).exists()
}

fn is_materializable_kind(entry: &Value) -> bool {
    matches!(entry.get("kind").and_then(Value::as_str), Some("vector" | "raster"))
}

fn append_bootstrap_app_icon(
    asset_ids: &mut BTreeSet<String>, project_dir: &Path, assets_dir: &Path,
    bootstrap: &BootstrapContext, doc: &Value, assets: &serde_json::Map<String, Value>,
) {
    if !bootstrap.triggers {
        return;
    }
    let Some(pointer) = doc.get("app-icon").and_then(Value::as_str) else {
        return;
    };
    let Some(entry) = assets.get(pointer) else {
        return;
    };
    if entry.get("role").and_then(Value::as_str) != Some("app-icon") {
        return;
    }

    let needs_materialize = bootstrap.missing_ui.iter().any(|platform| {
        if shell_resident_app_icon(project_dir, &platform.to_string()) {
            return false;
        }
        !bootstrap_platform_satisfied(assets_dir, entry, *platform)
    });
    if needs_materialize {
        asset_ids.insert(pointer.to_string());
    }
}

fn bootstrap_platform_satisfied(assets_dir: &Path, entry: &Value, platform: Platform) -> bool {
    let plat = platform.to_string();
    if let Some(pin) =
        entry.get("sources").and_then(|s| s.get(plat.as_str())).and_then(Value::as_str)
        && assets_dir.join(pin).exists()
    {
        return true;
    }
    if let Some(source) = entry.get("source").and_then(Value::as_str)
        && bootstrap_source_materializable(assets_dir, entry, source)
    {
        return true;
    }
    false
}

fn bootstrap_source_materializable(assets_dir: &Path, entry: &Value, source: &str) -> bool {
    if !assets_dir.join(source).is_file() {
        return false;
    }
    let kind = entry.get("kind").and_then(Value::as_str);
    let ext = Path::new(source).extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase);
    matches!(
        (kind, ext.as_deref()),
        (Some("vector"), Some("svg")) | (Some("raster"), Some("png" | "jpg" | "jpeg" | "webp"))
    )
}

fn asset_needs_materialize(
    entry: &Value, id: &str, assets_dir: &Path, shell_platforms: &[Platform],
) -> bool {
    if entry.get("role").and_then(Value::as_str) == Some("app-icon") {
        return shell_platforms.iter().any(|platform| {
            let plat = platform.to_string();
            !platform_pin_active(entry, &plat, assets_dir)
                && !app_icon_export_exists(assets_dir, &plat)
        });
    }
    let Some(kind) = entry.get("kind").and_then(Value::as_str) else {
        return false;
    };
    shell_platforms.iter().any(|platform| {
        let plat = platform.to_string();
        if platform_pin_active(entry, &plat, assets_dir) {
            return false;
        }
        if conventional_export_exists(assets_dir, id, kind, &plat) {
            return false;
        }
        entry.get("source").and_then(Value::as_str).is_some()
    })
}

fn conventional_export_exists(assets_dir: &Path, id: &str, kind: &str, platform: &str) -> bool {
    let exports_root = assets_dir.join("assets/exports").join(platform);
    if !exports_root.is_dir() {
        return false;
    }
    match (platform, kind) {
        ("ios", "vector" | "raster") => {
            let imageset = exports_root.join(format!("{id}.imageset"));
            imageset.is_dir() && directory_has_regular_file(&imageset)
        }
        ("android", "vector") => {
            exports_root.join("drawable").join(format!("{}.xml", kebab_to_snake(id))).is_file()
        }
        ("android", "raster") => android_raster_export_exists(&exports_root, id),
        _ => false,
    }
}

fn android_raster_export_exists(exports_root: &Path, id: &str) -> bool {
    let snake = kebab_to_snake(id);
    for density in ["mdpi", "hdpi", "xhdpi", "xxhdpi", "xxxhdpi"] {
        if exports_root.join(format!("drawable-{density}")).join(format!("{snake}.png")).is_file() {
            return true;
        }
        if exports_root.join(format!("mipmap-{density}")).join(format!("{snake}.png")).is_file() {
            return true;
        }
    }
    false
}

fn app_icon_export_exists(assets_dir: &Path, platform: &str) -> bool {
    let root = assets_dir.join(format!("assets/exports/{platform}/app-icon"));
    match platform {
        "ios" => {
            let appiconset = root.join("AppIcon.appiconset");
            appiconset.is_dir()
                && appiconset.join("Contents.json").is_file()
                && directory_has_extension(&appiconset, "png")
        }
        "android" => root.join("mipmap-anydpi-v26/ic_launcher.xml").is_file(),
        _ => false,
    }
}

fn kebab_to_snake(id: &str) -> String {
    id.replace('-', "_")
}

fn directory_has_regular_file(dir: &Path) -> bool {
    let Ok(entries) = fs::read_dir(dir) else {
        return false;
    };
    entries.filter_map(Result::ok).any(|entry| entry.path().is_file())
}

fn directory_has_extension(dir: &Path, ext: &str) -> bool {
    let Ok(entries) = fs::read_dir(dir) else {
        return false;
    };
    entries.filter_map(Result::ok).any(|entry| {
        entry
            .path()
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case(ext))
    })
}
