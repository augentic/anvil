//! Diagnostic-report wire envelope formatter.
//!
//! This is the only [`super::Format`] that validates against
//! [`specify_schema::DIAGNOSTIC_REPORT_JSON_SCHEMA`] before emit; the other
//! formatters are presentation layers driven by the same in-memory
//! [`DiagnosticReport`]. Pretty-printed JSON with a single trailing
//! newline keeps the output stable for byte-diff goldens.

use jsonschema::{Registry, Resource};
use serde_json::Value;
use specify_schema::{DIAGNOSTIC_JSON_SCHEMA, DIAGNOSTIC_REPORT_JSON_SCHEMA};

use super::RenderError;
use crate::diagnostic::DiagnosticReport;

const FINDING_SCHEMA_URL: &str =
    "https://github.com/augentic/specify/schemas/diagnostics/diagnostic.schema.json";

/// Render `report` as the diagnostic-report wire envelope.
///
/// # Errors
///
/// - [`RenderError::JsonSchemaValidation`] when the serialised
///   envelope fails [`specify_schema::DIAGNOSTIC_REPORT_JSON_SCHEMA`].
/// - [`RenderError::JsonSerialise`] when JSON (de)serialisation fails.
pub fn render(report: &DiagnosticReport) -> Result<String, RenderError> {
    let value = serde_json::to_value(report)?;
    render_value(&value)
}

/// Schema-validate `value` and emit it as pretty-printed JSON with a
/// trailing newline.
///
/// # Errors
///
/// - [`RenderError::JsonSchemaValidation`] when `value` fails the v1
///   envelope schema.
/// - [`RenderError::JsonSerialise`] when pretty-printing fails.
#[doc(hidden)]
pub fn render_value(value: &Value) -> Result<String, RenderError> {
    let validator = compile_envelope_validator()?;
    let errors: Vec<String> = validator.iter_errors(value).map(|err| err.to_string()).collect();
    if !errors.is_empty() {
        return Err(RenderError::JsonSchemaValidation {
            detail: errors.join("; "),
        });
    }
    let mut rendered = serde_json::to_string_pretty(value)?;
    rendered.push('\n');
    Ok(rendered)
}

fn compile_envelope_validator() -> Result<jsonschema::Validator, RenderError> {
    let envelope: Value = serde_json::from_str(DIAGNOSTIC_REPORT_JSON_SCHEMA)?;
    let finding: Value = serde_json::from_str(DIAGNOSTIC_JSON_SCHEMA)?;
    let registry = Registry::new()
        .add(FINDING_SCHEMA_URL, Resource::from_contents(finding))
        .and_then(jsonschema::RegistryBuilder::prepare)
        .map_err(|err| RenderError::JsonSchemaValidation {
            detail: format!("registry build failed: {err}"),
        })?;
    jsonschema::options().with_registry(&registry).build(&envelope).map_err(|err| {
        RenderError::JsonSchemaValidation {
            detail: format!("envelope schema compile failed: {err}"),
        }
    })
}
