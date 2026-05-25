use std::path::Path;

use serde_json::Value as JsonValue;

use tooling::schema::collect_errors_for_test;
use tooling::Context;

/// CLI-distributed JSON Schema basenames under `specify-cli/schemas/`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CliSchemaId {
    Evidence,
    Fusion,
}

impl CliSchemaId {
    pub const fn file_name(self) -> &'static str {
        match self {
            Self::Evidence => "evidence.schema.json",
            Self::Fusion => "slice/fusion.schema.json",
        }
    }
}

fn schema_path(ctx: &Context, schema_id: CliSchemaId) -> std::path::PathBuf {
    ctx.specify_cli_schemas_dir().join(schema_id.file_name())
}

/// Validate `data` against a CLI schema; soft-skip when the schema checkout is absent.
///
/// Mirrors `validateOrThrow()` in `tests/lib/validators.ts`.
pub fn validate_cli_schema_or_skip(
    ctx: &Context,
    schema_id: CliSchemaId,
    data: &JsonValue,
    context: &str,
) -> Result<(), String> {
    let path = schema_path(ctx, schema_id);
    if !path.is_file() {
        eprintln!(
            "  note: {} not found under SPECIFY_CLI_DIR; skipping schema check for {context}",
            schema_id.file_name()
        );
        return Ok(());
    }

    let compiled = ctx
        .schema(&path)
        .map_err(|err| format!("load schema {}: {err}", path.display()))?;

    collect_errors_for_test(&compiled, data).map_err(|errors| {
        let detail = errors
            .iter()
            .map(|error| {
                let path = if error.instance_path.is_empty() {
                    "/".to_string()
                } else {
                    error.instance_path.clone()
                };
                format!("{path} {}", error.message)
            })
            .collect::<Vec<_>>()
            .join("; ");
        format!("{context} fails {}: {detail}", schema_id.file_name())
    })
}

/// Convenience wrapper validating a YAML file on disk.
pub fn validate_yaml_file_or_skip(
    ctx: &Context,
    schema_id: CliSchemaId,
    path: impl AsRef<Path>,
) -> Result<(), String> {
    let path = path.as_ref();
    let data = super::read_yaml(path)?;
    validate_cli_schema_or_skip(ctx, schema_id, &data, &path.display().to_string())
}
