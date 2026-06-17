//! Bootstrap `app-icon` gate for `specify plan validate` (RFC-46 §6).

#[cfg(test)]
#[path = "bootstrap_app_icon/tests.rs"]
mod tests;

use std::path::Path;

use serde_json::Value;
use specify_diagnostics::{Diagnostic, Severity};
use specify_vectis_shell_detect::shell_resident_app_icon;

use crate::Platform;
use crate::change::plan::core::validate::plan_finding;
use crate::platform::bootstrap_context;

/// Stable code for the bootstrap `app-icon` gate (RFC §6.2).
pub const BOOTSTRAP_APP_ICON_MISSING: &str = "plan-bootstrap-app-icon-missing";

const ASSETS_REL: &str = "design-system/assets.yaml";

/// Emit `plan-bootstrap-app-icon-missing` when §6.1 triggers and any
/// missing UI platform fails §6.2.
pub fn detect(project_dir: &Path) -> Vec<Diagnostic> {
    let ctx = match bootstrap_context(project_dir) {
        Ok(ctx) => ctx,
        Err(err) => {
            return vec![plan_finding(
                BOOTSTRAP_APP_ICON_MISSING,
                Severity::Important,
                format!(
                    "bootstrap `app-icon` gate could not be evaluated (RFC §6.2): {err}; \
                     fix project configuration before plan validate or slice build prepare"
                ),
                None,
            )];
        }
    };
    if !ctx.triggers {
        return Vec::new();
    }

    let mut out = Vec::new();
    for platform in &ctx.missing_ui {
        if shell_resident_app_icon(project_dir, &platform.to_string()) {
            continue;
        }
        if let Some(message) = unsatisfied_reason(project_dir, *platform) {
            out.push(plan_finding(BOOTSTRAP_APP_ICON_MISSING, Severity::Important, message, None));
        }
    }
    out
}

fn unsatisfied_reason(project_dir: &Path, platform: Platform) -> Option<String> {
    let assets_path = project_dir.join(ASSETS_REL);
    if !assets_path.is_file() {
        return Some(missing_message(platform, "`design-system/assets.yaml` is absent"));
    }

    let Ok(raw) = std::fs::read_to_string(&assets_path) else {
        return Some(missing_message(
            platform,
            "`design-system/assets.yaml` exists but could not be read",
        ));
    };
    let Ok(doc) = serde_saphyr::from_str::<Value>(&raw) else {
        return Some(missing_message(platform, "`design-system/assets.yaml` is not valid YAML"));
    };
    let assets_dir = assets_path.parent().unwrap_or_else(|| Path::new("."));

    let Some(pointer) = doc.get("app-icon").and_then(Value::as_str) else {
        return Some(missing_message(
            platform,
            "top-level `app-icon` is absent from `design-system/assets.yaml`",
        ));
    };

    let Some(assets) = doc.get("assets").and_then(Value::as_object) else {
        return Some(missing_message(platform, "`design-system/assets.yaml` has no `assets:` map"));
    };

    let Some(entry) = assets.get(pointer) else {
        return Some(missing_message(
            platform,
            &format!(
                "top-level `app-icon` references unknown asset id `{pointer}` under `assets:`"
            ),
        ));
    };

    if entry.get("role").and_then(Value::as_str) != Some("app-icon") {
        return Some(missing_message(
            platform,
            &format!(
                "asset `{pointer}` referenced by top-level `app-icon` must have `role: app-icon`"
            ),
        ));
    }

    if platform_satisfied(assets_dir, entry, platform) {
        None
    } else {
        Some(missing_message(
            platform,
            "neither path A (materializable `source:` master on disk) nor path B \
             (operator-pinned export tree via `sources.<platform>`) satisfies RFC §4.1",
        ))
    }
}

pub(super) fn platform_satisfied(assets_dir: &Path, entry: &Value, platform: Platform) -> bool {
    let plat = platform.to_string();
    if let Some(pin) =
        entry.get("sources").and_then(|s| s.get(plat.as_str())).and_then(Value::as_str)
        && pin_resolves(assets_dir, pin)
    {
        return true;
    }
    if let Some(source) = entry.get("source").and_then(Value::as_str)
        && source_materializable(assets_dir, entry, source)
    {
        return true;
    }
    false
}

fn pin_resolves(assets_dir: &Path, path_rel: &str) -> bool {
    let resolved = assets_dir.join(path_rel);
    resolved.is_dir() || resolved.is_file()
}

pub(super) fn source_materializable(assets_dir: &Path, entry: &Value, source: &str) -> bool {
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

fn missing_message(platform: Platform, detail: &str) -> String {
    format!(
        "UI platform bootstrap requires a satisfiable `app-icon` for `{platform}` (RFC §6.2): \
         {detail}; provide path A (`source:` with a materializable SVG or square raster master) \
         or path B (operator-pinned export tree under `exports/{platform}/app-icon/` with \
         `sources.{platform}` pointing at the export root), or satisfy the shell-resident \
         launcher icon escape hatch (§6.3)"
    )
}
