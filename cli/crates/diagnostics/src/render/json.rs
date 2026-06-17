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
    "https://github.com/augentic/specify-cli/schemas/diagnostics/diagnostic.schema.json";

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

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::{render, render_value};
    use crate::diagnostic::{DiagnosticReport, DiagnosticSummary};
    use crate::render::RenderError;
    use crate::test_support::sample_diagnostic;

    const VALID_FINGERPRINT: &str =
        "sha256:0000000000000000000000000000000000000000000000000000000000000000";

    /// [`sample_diagnostic`] leaves `fingerprint` empty for the
    /// fingerprint tests; the JSON envelope schema requires the
    /// `sha256:<64 hex>` shape, so stamp a valid placeholder here.
    fn fingerprinted() -> crate::diagnostic::Diagnostic {
        let mut finding = sample_diagnostic();
        finding.fingerprint = VALID_FINGERPRINT.into();
        finding
    }

    fn report(findings: Vec<crate::diagnostic::Diagnostic>) -> DiagnosticReport {
        DiagnosticReport {
            version: crate::diagnostic::DiagnosticReportVersion,
            summary: DiagnosticSummary::from_diagnostics(&findings),
            findings,
        }
    }

    #[test]
    fn round_trips_to_pretty_json() {
        let out = render(&report(vec![fingerprinted()])).expect("valid envelope renders");
        assert!(out.ends_with('\n'), "single trailing newline");
        let parsed: Value = serde_json::from_str(&out).expect("output is valid JSON");
        assert_eq!(parsed["version"], json!(1));
        assert_eq!(parsed["summary"]["important"], json!(1));
        assert_eq!(parsed["findings"][0]["rule-id"], json!("UNI-014"));
    }

    #[test]
    fn empty_report_renders_valid_envelope() {
        let out = render(&report(vec![])).expect("empty envelope renders");
        let parsed: Value = serde_json::from_str(&out).expect("valid JSON");
        assert_eq!(parsed["findings"], json!([]));
    }

    #[test]
    fn schema_violation_render_error() {
        let bogus = json!({ "version": 1, "summary": {}, "findings": [{ "id": "x" }] });
        let err = render_value(&bogus).expect_err("incomplete finding must fail the schema");
        assert!(matches!(err, RenderError::JsonSchemaValidation { .. }));
    }

    /// A summary object missing one of its four tally keys fails the
    /// envelope schema before any bytes are emitted.
    #[test]
    fn rejects_missing_summary_key() {
        let bad = json!({
            "version": 1,
            "summary": { "critical": 0, "important": 0, "suggestion": 0 },
            "findings": []
        });
        let err = render_value(&bad).expect_err("missing summary key must be rejected");
        assert!(matches!(err, RenderError::JsonSchemaValidation { .. }));
    }

    /// The envelope preserves the producer's input order — the JSON
    /// `findings` array mirrors the slice order rather than re-sorting.
    #[test]
    fn preserves_input_finding_order() {
        let mut first = fingerprinted();
        first.id = "FIND-0001".into();
        first.title = "first".into();
        let mut second = fingerprinted();
        second.id = "FIND-0002".into();
        second.title = "second".into();

        let out = render(&report(vec![first, second])).expect("renders");
        let parsed: Value = serde_json::from_str(&out).expect("valid JSON");
        assert_eq!(parsed["findings"][0]["title"], json!("first"));
        assert_eq!(parsed["findings"][1]["title"], json!("second"));
    }
}
