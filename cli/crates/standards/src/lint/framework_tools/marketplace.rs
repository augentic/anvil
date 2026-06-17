//! In-process `marketplace` framework checker (Road B `kind: tool`).
//!
//! Covers CORE-022 (`plugins.marketplace-drift`): the
//! `.cursor-plugin/marketplace.json` manifest must satisfy the
//! canonical [`specify_schema::MARKETPLACE_JSON_SCHEMA`] and agree
//! bidirectionally with the on-disk `plugins/` layout. Carries no
//! policy — the schema is the canonical embedded constant and the
//! plugin layout is structural.

use std::collections::BTreeSet;
use std::path::Path;

use serde_json::Value as JsonValue;

use super::support::{ToolFinding, relative_display};

const RULE_MARKETPLACE_DRIFT: &str = "CORE-022";

const IMPACT: &str = "The marketplace manifest disagrees with the on-disk plugin layout, so the plugin set advertised to Cursor is wrong.";
const REMEDIATION: &str = "Reconcile .cursor-plugin/marketplace.json with the plugins/ tree: declare every on-disk plugin and ensure each declared plugin has skills/ and a plugin.json.";

/// Run the marketplace drift check (whole-tree; args carry no policy).
pub fn run(project_dir: &Path, _args: &[String]) -> Vec<ToolFinding> {
    check_marketplace_drift(project_dir)
}

fn drift(rel: &str, message: &str) -> ToolFinding {
    ToolFinding {
        rule_id: RULE_MARKETPLACE_DRIFT,
        path: Some(rel.to_string()),
        message: message.to_string(),
        impact: IMPACT,
        remediation: REMEDIATION,
    }
}

/// CORE-022: validate the marketplace manifest shape and its consistency
/// with the on-disk plugin layout rooted at `project_dir`.
fn check_marketplace_drift(project_dir: &Path) -> Vec<ToolFinding> {
    let manifest_path = project_dir.join(".cursor-plugin").join("marketplace.json");
    let manifest_rel = relative_display(project_dir, &manifest_path);

    // An adapters-only framework root (RFC-48 H1) carries no plugin
    // marketplace, so an absent manifest is a legitimate skip — distinct
    // from a present-but-unreadable manifest, which still flags below.
    if !manifest_path.exists() {
        return Vec::new();
    }

    let Ok(contents) = std::fs::read_to_string(&manifest_path) else {
        return vec![drift(&manifest_rel, "Cannot read .cursor-plugin/marketplace.json")];
    };

    let value: JsonValue = match serde_json::from_str(&contents) {
        Ok(value) => value,
        Err(error) => {
            return vec![drift(
                &manifest_rel,
                &format!("Cannot parse .cursor-plugin/marketplace.json: {error}"),
            )];
        }
    };

    let schema_findings = schema_findings(&value, &manifest_rel);
    if !schema_findings.is_empty() {
        return schema_findings;
    }

    let Some(plugins) = value.get("plugins").and_then(JsonValue::as_array) else {
        return vec![drift(&manifest_rel, "marketplace.json is missing a plugins array")];
    };

    let declared: BTreeSet<String> = plugins
        .iter()
        .filter_map(|entry| entry.get("source").and_then(JsonValue::as_str))
        .map(str::to_string)
        .collect();

    let plugins_dir = project_dir.join("plugins");
    let mut findings = undeclared_findings(project_dir, &plugins_dir, &declared);

    for plugin in plugins {
        let Some(name) = plugin.get("name").and_then(JsonValue::as_str) else {
            continue;
        };
        let Some(source) = plugin.get("source").and_then(JsonValue::as_str) else {
            continue;
        };

        let plugin_root = plugins_dir.join(source);
        let skills_dir = plugin_root.join("skills");
        match std::fs::metadata(&skills_dir) {
            Ok(metadata) if metadata.is_dir() => {}
            Ok(_) => {
                findings.push(drift(
                    &manifest_rel,
                    &format!("Plugin '{name}' has no skills/ directory"),
                ));
                continue;
            }
            Err(_) => {
                findings.push(drift(
                    &manifest_rel,
                    &format!("Plugin '{name}' declared in marketplace.json but skills/ not found"),
                ));
                continue;
            }
        }

        let plugin_manifest = plugin_root.join(".cursor-plugin").join("plugin.json");
        let plugin_manifest_rel = relative_display(project_dir, &plugin_manifest);
        match std::fs::metadata(&plugin_manifest) {
            Ok(metadata) if metadata.is_file() => {}
            Ok(_) => findings.push(drift(
                &plugin_manifest_rel,
                &format!(
                    "Plugin '{name}' has skills/ but .cursor-plugin/plugin.json is not a file"
                ),
            )),
            Err(_) => findings.push(drift(
                &plugin_manifest_rel,
                &format!("Plugin '{name}' has skills/ but .cursor-plugin/plugin.json not found"),
            )),
        }
    }

    findings
}

/// Validate the parsed manifest against the canonical marketplace
/// schema, returning one finding per constraint violation (or an
/// infrastructure finding when the schema itself cannot compile).
fn schema_findings(value: &JsonValue, manifest_rel: &str) -> Vec<ToolFinding> {
    let validator = match specify_schema::cached_validator(specify_schema::MARKETPLACE_JSON_SCHEMA)
    {
        Ok(validator) => validator,
        Err(error) => {
            return vec![drift(
                manifest_rel,
                &format!("Cannot validate .cursor-plugin/marketplace.json: {error}"),
            )];
        }
    };
    if validator.is_valid(value) {
        return Vec::new();
    }
    validator
        .iter_errors(value)
        .map(|error| {
            drift(
                manifest_rel,
                &format!(
                    "marketplace.json schema violation at {}: {}",
                    error.instance_path(),
                    error
                ),
            )
        })
        .collect()
}

/// Flag every on-disk `plugins/<plugin>/.cursor-plugin/plugin.json` whose
/// `<plugin>` directory name is not a declared `source`.
fn undeclared_findings(
    project_dir: &Path, plugins_dir: &Path, declared: &BTreeSet<String>,
) -> Vec<ToolFinding> {
    let Ok(entries) = std::fs::read_dir(plugins_dir) else {
        return Vec::new();
    };
    let mut findings = Vec::new();
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() || !file_type.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let manifest = entry.path().join(".cursor-plugin").join("plugin.json");
        if !manifest.is_file() {
            continue;
        }
        if !declared.contains(&name) {
            let rel = relative_display(project_dir, &manifest);
            findings.push(drift(
                &rel,
                &format!(
                    "Plugin '{name}' has .cursor-plugin/plugin.json but is not in marketplace.json"
                ),
            ));
        }
    }
    findings.sort_by(|a, b| a.message.cmp(&b.message));
    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_MANIFEST: &str = r#"{
  "name": "test",
  "owner": { "name": "Test Owner", "email": "test@example.com" },
  "metadata": { "description": "Synthetic.", "version": "0.0.0", "pluginRoot": "plugins" },
  "plugins": [
    { "name": "spec", "source": "spec", "description": "Spec plugin." }
  ]
}
"#;

    fn write(root: &Path, rel: &str, body: &str) {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        std::fs::write(path, body).expect("write");
    }

    fn write_plugin(root: &Path, source: &str) {
        write(root, &format!("plugins/{source}/skills/.keep"), "");
        write(root, &format!("plugins/{source}/.cursor-plugin/plugin.json"), "{\"name\":\"x\"}");
    }

    #[test]
    fn clean_tree_is_silent() {
        let dir = tempfile::tempdir().expect("tempdir");
        write(dir.path(), ".cursor-plugin/marketplace.json", VALID_MANIFEST);
        write_plugin(dir.path(), "spec");
        assert!(check_marketplace_drift(dir.path()).is_empty());
    }

    #[test]
    fn absent_manifest_is_silent() {
        let dir = tempfile::tempdir().expect("tempdir");
        // An adapters-only framework root (RFC-48 H1) has no
        // marketplace.json at all: absent is a skip, not drift.
        assert!(check_marketplace_drift(dir.path()).is_empty());
    }

    #[test]
    fn flags_undeclared_plugin() {
        let dir = tempfile::tempdir().expect("tempdir");
        write(dir.path(), ".cursor-plugin/marketplace.json", VALID_MANIFEST);
        write_plugin(dir.path(), "spec");
        write_plugin(dir.path(), "orphan");
        let findings = check_marketplace_drift(dir.path());
        assert!(
            findings
                .iter()
                .any(|f| f.message.contains("orphan")
                    && f.message.contains("not in marketplace.json")),
            "expected undeclared plugin finding, got {findings:?}"
        );
    }

    #[test]
    fn flags_schema_violation() {
        let dir = tempfile::tempdir().expect("tempdir");
        write(
            dir.path(),
            ".cursor-plugin/marketplace.json",
            "{\n  \"name\": \"bad\",\n  \"plugins\": []\n}\n",
        );
        let findings = check_marketplace_drift(dir.path());
        assert!(
            findings.iter().all(|f| f.rule_id == RULE_MARKETPLACE_DRIFT),
            "all findings carry CORE-022"
        );
        assert!(
            findings.iter().any(|f| f.message.contains("schema violation")),
            "expected schema violation finding, got {findings:?}"
        );
    }

    #[test]
    fn flags_declared_plugin_missing_skills() {
        let dir = tempfile::tempdir().expect("tempdir");
        write(dir.path(), ".cursor-plugin/marketplace.json", VALID_MANIFEST);
        write(dir.path(), "plugins/spec/.cursor-plugin/plugin.json", "{\"name\":\"x\"}");
        let findings = check_marketplace_drift(dir.path());
        assert!(
            findings.iter().any(|f| f.message.contains("skills/ not found")),
            "expected missing-skills finding, got {findings:?}"
        );
    }
}
