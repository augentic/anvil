use std::fs;
use std::path::{Component, Path};

use serde_json::Value as JsonValue;
use walkdir::WalkDir;

use crate::context::Context;
use crate::finding::{Check, Finding, Location};
use crate::schema::{validate_value, SchemaError, SchemaId};

const RULE_BROKEN_SYMLINK: &str = "plugins.broken-symlink";
const RULE_MARKETPLACE_DRIFT: &str = "plugins.marketplace-drift";

/// Verify every symlink under `plugins/` resolves.
pub struct BrokenSymlinkCheck;

/// Validate marketplace manifest shape and plugin directory consistency.
pub struct MarketplaceDriftCheck;

impl Check for BrokenSymlinkCheck {
    fn id(&self) -> &'static str {
        RULE_BROKEN_SYMLINK
    }

    fn run(&self, ctx: &Context) -> Vec<Finding> {
        check_symlinks(ctx)
    }
}

impl Check for MarketplaceDriftCheck {
    fn id(&self) -> &'static str {
        RULE_MARKETPLACE_DRIFT
    }

    fn run(&self, ctx: &Context) -> Vec<Finding> {
        check_marketplace_consistency(ctx)
    }
}

fn check_symlinks(ctx: &Context) -> Vec<Finding> {
    let plugins_dir = ctx.plugins_dir();
    if !plugins_dir.is_dir() {
        return Vec::new();
    }

    let framework_root = ctx.framework_root();
    let mut findings = Vec::new();

    for entry in WalkDir::new(&plugins_dir).follow_links(false) {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };

        let path = entry.path();
        let metadata = match fs::symlink_metadata(path) {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };
        if !metadata.file_type().is_symlink() {
            continue;
        }

        if fs::metadata(path).is_err() {
            let rel = path
                .strip_prefix(framework_root)
                .unwrap_or(path)
                .display();
            findings.push(Finding {
                rule_id: RULE_BROKEN_SYMLINK,
                message: format!("Broken symlink: {rel}"),
                location: Some(location(path)),
            });
        }
    }

    findings
}

fn check_marketplace_consistency(ctx: &Context) -> Vec<Finding> {
    let manifest_path = ctx
        .framework_root()
        .join(".cursor-plugin")
        .join("marketplace.json");
    let plugins_dir = ctx.plugins_dir();

    let contents = match fs::read_to_string(&manifest_path) {
        Ok(contents) => contents,
        Err(_) => {
            return vec![drift_finding(
                "Cannot read .cursor-plugin/marketplace.json",
                &manifest_path,
            )];
        }
    };

    let value: JsonValue = match serde_json::from_str(&contents) {
        Ok(value) => value,
        Err(error) => {
            return vec![drift_finding(
                format!("Cannot parse .cursor-plugin/marketplace.json: {error}"),
                &manifest_path,
            )];
        }
    };

    let mut findings = schema_findings(ctx, &value, &manifest_path);
    if !findings.is_empty() {
        return findings;
    }

    let Some(plugins) = value.get("plugins").and_then(JsonValue::as_array) else {
        return vec![drift_finding(
            "marketplace.json is missing a plugins array",
            &manifest_path,
        )];
    };

    let declared_sources: std::collections::HashSet<String> = plugins
        .iter()
        .filter_map(|entry| entry.get("source").and_then(JsonValue::as_str))
        .map(str::to_string)
        .collect();

    findings.extend(undeclared_plugin_findings(&plugins_dir, &declared_sources));

    for plugin in plugins {
        let Some(name) = plugin.get("name").and_then(JsonValue::as_str) else {
            continue;
        };
        let Some(source) = plugin.get("source").and_then(JsonValue::as_str) else {
            continue;
        };

        let plugin_dir = plugins_dir.join(source);
        let skills_dir = plugin_dir.join("skills");

        let skills_metadata = match fs::metadata(&skills_dir) {
            Ok(metadata) => metadata,
            Err(_) => {
                findings.push(drift_finding(
                    format!(
                        "Plugin '{name}' declared in marketplace.json but skills/ not found"
                    ),
                    &manifest_path,
                ));
                continue;
            }
        };

        if !skills_metadata.is_dir() {
            findings.push(drift_finding(
                format!("Plugin '{name}' has no skills/ directory"),
                &manifest_path,
            ));
            continue;
        }

        let plugin_manifest_path = plugin_dir.join(".cursor-plugin").join("plugin.json");
        match fs::metadata(&plugin_manifest_path) {
            Ok(metadata) if metadata.is_file() => {}
            Ok(_) => findings.push(drift_finding(
                format!(
                    "Plugin '{name}' has skills/ but .cursor-plugin/plugin.json is not a file"
                ),
                &plugin_manifest_path,
            )),
            Err(_) => findings.push(drift_finding(
                format!(
                    "Plugin '{name}' has skills/ but .cursor-plugin/plugin.json not found"
                ),
                &plugin_manifest_path,
            )),
        }
    }

    findings
}

fn schema_findings(ctx: &Context, value: &JsonValue, manifest_path: &Path) -> Vec<Finding> {
    match validate_value(ctx, value, SchemaId::Marketplace) {
        Ok(()) => Vec::new(),
        Err(SchemaError::Infrastructure(error)) => vec![drift_finding(
            format!("Cannot validate .cursor-plugin/marketplace.json: {error}"),
            manifest_path,
        )],
        Err(SchemaError::Validation(errors)) => errors
            .into_iter()
            .map(|error| {
                drift_finding(
                    format!(
                        "marketplace.json schema violation at {}: {}",
                        error.instance_path, error.message
                    ),
                    manifest_path,
                )
            })
            .collect(),
    }
}

fn undeclared_plugin_findings(
    plugins_dir: &Path,
    declared_sources: &std::collections::HashSet<String>,
) -> Vec<Finding> {
    if !plugins_dir.is_dir() {
        return Vec::new();
    }

    let mut findings = Vec::new();

    for entry in WalkDir::new(plugins_dir)
        .max_depth(3)
        .follow_links(false)
    {
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => continue,
        };
        if !entry.file_type().is_file() {
            continue;
        }
        if entry.file_name() != "plugin.json" {
            continue;
        }

        let path = entry.into_path();
        let rel = match path.strip_prefix(plugins_dir) {
            Ok(rel) => rel,
            Err(_) => continue,
        };

        let mut components = rel.components();
        let Some(Component::Normal(plugin_dir)) = components.next() else {
            continue;
        };
        let Some(Component::Normal(cursor_plugin)) = components.next() else {
            continue;
        };
        let Some(Component::Normal(file)) = components.next() else {
            continue;
        };
        if components.next().is_some() {
            continue;
        }
        if cursor_plugin != ".cursor-plugin" || file != "plugin.json" {
            continue;
        }

        let plugin_dir = plugin_dir.to_string_lossy();
        if !declared_sources.contains(plugin_dir.as_ref()) {
            findings.push(drift_finding(
                format!(
                    "Plugin '{plugin_dir}' has .cursor-plugin/plugin.json but is not in marketplace.json"
                ),
                &path,
            ));
        }
    }

    findings
}

fn drift_finding(message: impl Into<String>, path: &Path) -> Finding {
    Finding {
        rule_id: RULE_MARKETPLACE_DRIFT,
        message: message.into(),
        location: Some(location(path)),
    }
}

fn location(path: &Path) -> Location {
    Location {
        path: path.to_path_buf(),
        line: 1,
        column: None,
    }
}