use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use jsonschema::Validator;
use serde_json::Value as JsonValue;

use crate::context::Context;
use crate::error::ToolingError;
use crate::helpers::skill_frontmatter;

/// Framework-only JSON Schema identifiers under `tooling/schemas/`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SchemaId {
    Skill,
    CodexRule,
    Scenario,
    Marketplace,
}

impl SchemaId {
    /// Basename of the schema file in `tooling/schemas/`.
    pub const fn file_name(self) -> &'static str {
        match self {
            Self::Skill => "skill.schema.json",
            Self::CodexRule => "codex-rule.schema.json",
            Self::Scenario => "scenario.schema.json",
            Self::Marketplace => "marketplace.schema.json",
        }
    }
}

/// Schema validation failure: infrastructure problem or one or more constraint violations.
#[derive(Debug)]
pub enum SchemaError {
    Infrastructure(ToolingError),
    Validation(Vec<ValidationError>),
}

impl From<ToolingError> for SchemaError {
    fn from(error: ToolingError) -> Self {
        Self::Infrastructure(error)
    }
}

/// One JSON Schema validation error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    pub instance_path: String,
    pub message: String,
}

/// Resolve the authoritative schema path for `schema_id`.
pub fn schema_path(ctx: &Context, schema_id: SchemaId) -> PathBuf {
    ctx.framework_schema_dir().join(schema_id.file_name())
}

/// Lazily compile a framework schema via the shared context cache.
pub fn validator(ctx: &Context, schema_id: SchemaId) -> Result<std::sync::Arc<Validator>, ToolingError> {
    ctx.schema(schema_path(ctx, schema_id))
}

/// Validate a parsed JSON/YAML value against a framework schema.
pub fn validate_value(
    ctx: &Context,
    value: &JsonValue,
    schema_id: SchemaId,
) -> Result<(), SchemaError> {
    let compiled = validator(ctx, schema_id)?;
    collect_errors(&compiled, value).map_err(SchemaError::Validation)
}

/// Extract YAML frontmatter from a Markdown file and validate it against `schema_id`.
pub fn validate_frontmatter(
    ctx: &Context,
    path: impl AsRef<Path>,
    schema_id: SchemaId,
) -> Result<(), SchemaError> {
    let path = path.as_ref();
    let content = fs::read_to_string(path).map_err(|source| {
        SchemaError::Infrastructure(ToolingError::Infrastructure(format!(
            "read {}: {source}",
            path.display()
        )))
    })?;

    let Some(frontmatter) = skill_frontmatter(&content) else {
        return Err(SchemaError::Validation(vec![ValidationError {
            instance_path: "/".into(),
            message: "missing leading YAML frontmatter delimited by ---".into(),
        }]));
    };

    let value = frontmatter_to_json(frontmatter);
    validate_value(ctx, &value, schema_id)
}

fn frontmatter_to_json(frontmatter: BTreeMap<String, JsonValue>) -> JsonValue {
    JsonValue::Object(frontmatter.into_iter().collect())
}

fn collect_errors(compiled: &Validator, value: &JsonValue) -> Result<(), Vec<ValidationError>> {
    if compiled.is_valid(value) {
        return Ok(());
    }

    let errors = compiled
        .iter_errors(value)
        .map(|error| ValidationError {
            instance_path: error.instance_path.to_string(),
            message: error.to_string(),
        })
        .collect();

    Err(errors)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn validate_frontmatter_rejects_invalid_skill_description() {
        let ctx =
            Context::from_manifest_dir(env!("CARGO_MANIFEST_DIR")).expect("framework root resolves");
        let mut temp = tempfile::NamedTempFile::new().expect("temp file");
        write!(
            temp,
            "---\nname: spec-test-skill\ndescription: Too short.\n---\n"
        )
        .expect("write temp frontmatter");

        let result = validate_frontmatter(&ctx, temp.path(), SchemaId::Skill);
        let SchemaError::Validation(errors) = result.expect_err("invalid frontmatter should fail")
        else {
            panic!("expected validation errors");
        };
        assert!(
            errors.iter().any(|error| {
                error.instance_path.contains("description")
                    || error.message.contains("Use when")
            }),
            "expected description validation error, got {errors:?}"
        );
    }

    #[test]
    fn validate_frontmatter_accepts_minimal_valid_skill() {
        let ctx =
            Context::from_manifest_dir(env!("CARGO_MANIFEST_DIR")).expect("framework root resolves");
        let skill_path = ctx
            .plugins_dir()
            .join("spec/skills/init/SKILL.md");
        validate_frontmatter(&ctx, &skill_path, SchemaId::Skill)
            .unwrap_or_else(|error| panic!("spec-init should validate: {error:?}"));
    }
}
