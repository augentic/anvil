//! Prose and manifest predicates: marketplace↔plugins drift and the
//! canonical review-team-protocol document.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use serde_json::Value as JsonValue;

use crate::support::{Finding, rel};

/// The marketplace manifest disagrees with the on-disk plugin layout.
pub const CHECK_MARKETPLACE_DRIFT: &str = "plugins.marketplace-drift";
/// The canonical review-team-protocol document is missing.
pub const CHECK_CANONICAL_MISSING: &str = "agent-teams.missing-canonical";

/// Canonical documents required by shipped overlays: the sibling
/// adapter repo symlinks `agent-teams.md` through this file.
const CANONICAL_DOCUMENTS: &[&str] = &["docs/reference/review-team-protocol.md"];

/// Run every prose predicate rooted at `root`.
pub fn run(root: &Path) -> Vec<Finding> {
    let mut findings = Vec::new();
    check_marketplace_drift(root, &mut findings);
    check_canonical_doc(root, &mut findings);
    findings
}

const fn marketplace_drift(message: String) -> Finding {
    Finding::new(CHECK_MARKETPLACE_DRIFT, message)
}

/// The `.cursor-plugin/marketplace.json` manifest must satisfy the
/// embedded marketplace schema and agree bidirectionally with the
/// on-disk `plugins/` layout (an absent manifest is a legitimate skip).
fn check_marketplace_drift(root: &Path, findings: &mut Vec<Finding>) {
    let manifest_path = root.join(".cursor-plugin").join("marketplace.json");
    if !manifest_path.exists() {
        return;
    }
    let manifest_rel = rel(root, &manifest_path);
    let Ok(contents) = fs::read_to_string(&manifest_path) else {
        findings.push(marketplace_drift(format!("{manifest_rel} — cannot read manifest")));
        return;
    };
    let value: JsonValue = match serde_json::from_str(&contents) {
        Ok(value) => value,
        Err(error) => {
            findings.push(marketplace_drift(format!("{manifest_rel} — cannot parse: {error}")));
            return;
        }
    };

    match schema::cached_validator(schema::MARKETPLACE_JSON_SCHEMA) {
        Ok(validator) => {
            let errors: Vec<Finding> = validator
                .iter_errors(&value)
                .map(|error| {
                    marketplace_drift(format!(
                        "{manifest_rel} — schema violation at {}: {error}",
                        error.instance_path()
                    ))
                })
                .collect();
            if !errors.is_empty() {
                findings.extend(errors);
                return;
            }
        }
        Err(error) => {
            findings.push(marketplace_drift(format!(
                "{manifest_rel} — cannot compile marketplace schema: {error}"
            )));
            return;
        }
    }

    let Some(plugins) = value.get("plugins").and_then(JsonValue::as_array) else {
        findings.push(marketplace_drift(format!("{manifest_rel} — missing plugins array")));
        return;
    };
    let declared: BTreeSet<&str> =
        plugins.iter().filter_map(|p| p.get("source").and_then(JsonValue::as_str)).collect();

    let plugins_dir = root.join("plugins");
    if let Ok(entries) = fs::read_dir(&plugins_dir) {
        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_symlink() || !file_type.is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            if entry.path().join(".cursor-plugin").join("plugin.json").is_file()
                && !declared.contains(name.as_str())
            {
                findings.push(marketplace_drift(format!(
                    "plugin '{name}' has .cursor-plugin/plugin.json but is not in \
                     marketplace.json"
                )));
            }
        }
    }

    for plugin in plugins {
        let Some(name) = plugin.get("name").and_then(JsonValue::as_str) else {
            continue;
        };
        let Some(source) = plugin.get("source").and_then(JsonValue::as_str) else {
            continue;
        };
        let plugin_root = plugins_dir.join(source);
        if !plugin_root.join("skills").is_dir() {
            findings.push(marketplace_drift(format!(
                "plugin '{name}' declared in marketplace.json but skills/ not found"
            )));
            continue;
        }
        if !plugin_root.join(".cursor-plugin").join("plugin.json").is_file() {
            findings.push(marketplace_drift(format!(
                "plugin '{name}' has skills/ but .cursor-plugin/plugin.json not found"
            )));
        }
    }
}

/// Canonical overlay documents must exist.
fn check_canonical_doc(root: &Path, findings: &mut Vec<Finding>) {
    for relative in CANONICAL_DOCUMENTS {
        if !root.join(relative).is_file() {
            findings.push(Finding::new(
                CHECK_CANONICAL_MISSING,
                format!("required file '{relative}' is missing"),
            ));
        }
    }
}
