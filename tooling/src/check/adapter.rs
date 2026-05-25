use std::fs;
use std::path::Path;

use jsonschema::ValidationError;
use serde_json::Value as JsonValue;

use crate::context::{specify_cli_schemas_available, specify_cli_setup_hint, Context};
use crate::finding::{Check, Finding, Location};
use crate::helpers::{under_symlink, walk_matching_files};

pub const RULE_SCHEMA_VIOLATION: &str = "adapter.schema-violation";
pub const RULE_MISSING_MANIFEST: &str = "adapter.missing-manifest";
const ADAPTER_FILENAME: &str = "adapter.yaml";

/// Adapter manifest validation against `specify-cli` runtime schemas.
pub struct AdapterCheck;

impl Check for AdapterCheck {
    fn run(&self, ctx: &Context) -> Vec<Finding> {
        run_adapter_check(ctx)
    }
}

pub fn run_adapter_check(ctx: &Context) -> Vec<Finding> {
    let mut findings = Vec::new();

    findings.extend(check_missing_manifests(ctx, &ctx.sources_dir()));
    findings.extend(check_missing_manifests(ctx, &ctx.targets_dir()));

    match load_runtime_validator(ctx, "source.schema.json") {
        Ok(validator) => {
            findings.extend(validate_manifests(
                ctx,
                &validator,
                &ctx.sources_dir(),
                "source.schema.json",
            ));
        }
        Err(finding) => findings.push(finding),
    }

    match load_runtime_validator(ctx, "target.schema.json") {
        Ok(validator) => {
            findings.extend(validate_manifests(
                ctx,
                &validator,
                &ctx.targets_dir(),
                "target.schema.json",
            ));
        }
        Err(finding) => findings.push(finding),
    }

    findings
}

fn check_missing_manifests(ctx: &Context, axis_dir: &Path) -> Vec<Finding> {
    let Ok(entries) = fs::read_dir(axis_dir) else {
        return Vec::new();
    };

    let mut findings = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if under_symlink(ctx.framework_root(), &path).unwrap_or(true) {
            continue;
        }
        let manifest = path.join(ADAPTER_FILENAME);
        if manifest.is_file() {
            continue;
        }
        let rel = relative_path(ctx, &path);
        findings.push(Finding {
            rule_id: RULE_MISSING_MANIFEST,
            message: format!(
                "Adapter directory missing manifest: {rel} — expected {ADAPTER_FILENAME}"
            ),
            location: Some(Location {
                path: path.clone(),
                line: 1,
                column: None,
            }),
        });
    }
    findings.sort_by(|a, b| a.message.cmp(&b.message));
    findings
}

fn validate_manifests(
    ctx: &Context,
    validator: &jsonschema::Validator,
    axis_dir: &Path,
    schema_file: &str,
) -> Vec<Finding> {
    let Ok(paths) = walk_matching_files(ctx.framework_root(), axis_dir, ADAPTER_FILENAME) else {
        return Vec::new();
    };

    let mut findings = Vec::new();
    for path in paths {
        findings.extend(validate_manifest(ctx, validator, &path, schema_file));
    }
    findings
}

fn validate_manifest(
    ctx: &Context,
    validator: &jsonschema::Validator,
    path: &Path,
    _schema_file: &str,
) -> Vec<Finding> {
    let rel = relative_path(ctx, path);
    let raw = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(source) => {
            return vec![schema_finding(
                path,
                format!("Adapter validation failed: {rel} — read failed: {source}"),
            )];
        }
    };

    let value: JsonValue = match serde_saphyr::from_str(&raw) {
        Ok(value) => value,
        Err(source) => {
            return vec![schema_finding(
                path,
                format!("Adapter validation failed: {rel} — YAML parse failed: {source}"),
            )];
        }
    };

    if validator.is_valid(&value) {
        return Vec::new();
    }

    validator
        .iter_errors(&value)
        .map(|error| {
            schema_finding(
                path,
                format!(
                    "Adapter validation failed: {rel} — {}",
                    format_schema_error(&error)
                ),
            )
        })
        .collect()
}

fn load_runtime_validator(
    ctx: &Context,
    schema_file: &str,
) -> Result<std::sync::Arc<jsonschema::Validator>, Finding> {
    let specify_cli_dir = ctx.specify_cli_dir();
    if !specify_cli_schemas_available(&specify_cli_dir) {
        return Err(Finding {
            rule_id: RULE_SCHEMA_VIOLATION,
            message: specify_cli_setup_hint(&specify_cli_dir),
            location: None,
        });
    }

    let path = ctx.specify_cli_schemas_dir().join(schema_file);
    ctx.schema(&path).map_err(|error| Finding {
        rule_id: RULE_SCHEMA_VIOLATION,
        message: format!(
            "Adapter validation failed: could not load runtime schema {}: {error}",
            path.display()
        ),
        location: None,
    })
}

fn schema_finding(path: &Path, message: String) -> Finding {
    Finding {
        rule_id: RULE_SCHEMA_VIOLATION,
        message,
        location: Some(Location {
            path: path.to_path_buf(),
            line: 1,
            column: None,
        }),
    }
}

fn relative_path(ctx: &Context, path: &Path) -> String {
    path.strip_prefix(ctx.framework_root())
        .unwrap_or(path)
        .display()
        .to_string()
}

/// Mirror `formatSchemaError()` from `scripts/checks/_shared.ts`.
fn format_schema_error(error: &ValidationError<'_>) -> String {
    use jsonschema::error::ValidationErrorKind;

    let at = {
        let path = error.instance_path.to_string();
        if path.is_empty() { "/".to_string() } else { path }
    };

    match &error.kind {
        ValidationErrorKind::Required { property } => {
            let missing = property
                .as_str()
                .map(str::to_string)
                .unwrap_or_else(|| property.to_string());
            format!("{at} missing required property '{missing}'")
        }
        ValidationErrorKind::AdditionalProperties { unexpected } => {
            let property = unexpected.first().map_or("?", String::as_str);
            format!("{at} unknown property '{property}'")
        }
        ValidationErrorKind::Enum { options } => {
            let allowed = format_allowed_values(options);
            format!("{at} must be one of {allowed}")
        }
        ValidationErrorKind::Pattern { pattern } => {
            format!("{at} must match {pattern}")
        }
        _ => {
            let message = error.to_string();
            format!("{at} {message}").trim().to_string()
        }
    }
}

fn format_allowed_values(options: &JsonValue) -> String {
    let Some(values) = options.as_array() else {
        return options.to_string();
    };
    values
        .iter()
        .map(|value| {
            if value.is_string() {
                serde_json::to_string(value).unwrap_or_else(|_| value.to_string())
            } else {
                value.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_schema_error_required_matches_deno_shape() {
        let schema = serde_json::json!({
            "type": "object",
            "required": ["name"],
            "properties": { "name": { "type": "string" } }
        });
        let validator = jsonschema::validator_for(&schema).expect("schema compiles");
        let instance = serde_json::json!({});
        let error = validator
            .iter_errors(&instance)
            .next()
            .expect("required error");
        assert_eq!(
            format_schema_error(&error),
            "/ missing required property 'name'"
        );
    }

    #[test]
    fn format_schema_error_additional_property_matches_deno_shape() {
        let schema = serde_json::json!({
            "type": "object",
            "additionalProperties": false,
            "properties": { "name": { "type": "string" } }
        });
        let validator = jsonschema::validator_for(&schema).expect("schema compiles");
        let instance = serde_json::json!({ "extra": true });
        let error = validator
            .iter_errors(&instance)
            .next()
            .expect("additionalProperties error");
        assert_eq!(
            format_schema_error(&error),
            "/ unknown property 'extra'"
        );
    }

    #[test]
    fn relative_path_strips_framework_root() {
        let ctx =
            Context::from_manifest_dir(env!("CARGO_MANIFEST_DIR")).expect("framework root resolves");
        let path = ctx.sources_dir().join("intent").join(ADAPTER_FILENAME);
        assert_eq!(
            relative_path(&ctx, &path),
            "adapters/sources/intent/adapter.yaml"
        );
    }

    #[test]
    fn missing_manifest_detects_empty_adapter_directory() {
        let temp = tempfile::tempdir().expect("tempdir");
        scaffold_framework(temp.path());
        let adapter_dir = temp.path().join("adapters/sources/broken");
        fs::create_dir_all(&adapter_dir).expect("adapter dir");
        let ctx = Context::from_framework_root(temp.path()).expect("context");
        let findings = check_missing_manifests(&ctx, &ctx.sources_dir());
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, RULE_MISSING_MANIFEST);
        assert!(findings[0].message.contains("adapters/sources/broken"));
    }

    fn scaffold_framework(root: &Path) {
        fs::create_dir_all(root.join("plugins")).expect("plugins");
        fs::create_dir_all(root.join("adapters/sources")).expect("sources");
        fs::create_dir_all(root.join("adapters/targets")).expect("targets");
        fs::create_dir_all(root.join("tooling")).expect("tooling");
        fs::write(root.join("tooling/Cargo.toml"), "[package]\nname = \"tooling\"\n")
            .expect("cargo.toml");
    }
}
