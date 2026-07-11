//! Shared kernels behind the `validate_*` entry points: parse-then-
//! validate plumbing and failure folding.

use error::{Error, Result};
use schema::{
    ValidationStatus, ValidationSummary, Validator, join_details, validate_value_cached,
    validation_errors,
};
use serde_json::Value as JsonValue;

/// Prefix a validation summary's detail with a caller-supplied label
/// (a file path or lead id) so the operator can find the offending
/// document.
pub(super) fn relabel(
    mut summary: ValidationSummary, label: impl std::fmt::Display,
) -> ValidationSummary {
    let detail = summary.detail.take().unwrap_or_default();
    summary.detail =
        Some(if detail.is_empty() { label.to_string() } else { format!("{label}: {detail}") });
    summary
}

/// Parse `content` (JSON or its YAML superset) and validate it against a
/// `$ref`-free embedded `schema`, folding every schema failure into one
/// payload-free [`Error::Validation`] keyed on `code`.
///
/// This is the shared kernel behind the simple `validate_*_json`
/// entry points whose schema carries no relative `$ref`.
///
/// # Errors
///
/// Returns [`Error::Validation`] (keyed on `code`) when parsing or
/// schema validation fails.
pub(super) fn validate_parsed_json(
    content: &str, schema: &'static str, code: &'static str, rule: &str,
) -> Result<()> {
    let instance: JsonValue = serde_saphyr::from_str(content)
        .map_err(|err| Error::validation_failed(code, rule, format!("parse failed: {err}")))?;
    err_from_failures(code, &validation_failures(&instance, schema, code, rule))
}

/// Parse YAML `content` and validate it against a `$ref`-free embedded
/// `schema`, labelling the parse error and every schema failure with
/// `source_path` so an operator can find the offending file, then
/// folding the failures into one [`Error::Validation`] keyed on `code`.
///
/// This is the shared kernel behind the file-anchored `validate_*_yaml`
/// / `validate_evidence` entry points.
///
/// # Errors
///
/// Returns [`Error::Validation`] (keyed on `code`) when parsing or
/// schema validation fails.
pub(super) fn validate_labelled_yaml(
    content: &str, source_path: &std::path::Path, schema: &'static str, code: &'static str,
    rule: &str,
) -> Result<()> {
    let instance: JsonValue = serde_saphyr::from_str(content).map_err(|err| {
        Error::validation_failed(
            code,
            rule,
            format!("{}: YAML parse failed: {err}", source_path.display()),
        )
    })?;
    let failures: Vec<ValidationSummary> = validation_failures(&instance, schema, code, rule)
        .into_iter()
        .map(|summary| relabel(summary, source_path.display()))
        .collect();
    err_from_failures(code, &failures)
}

/// Parse `content` and validate it against a pre-compiled, registry-backed
/// `validator` (one whose schema carries a relative `$ref`), folding every
/// schema failure into one [`Error::Validation`] keyed on `code`.
///
/// # Errors
///
/// Returns [`Error::Validation`] (keyed on `code`) when parsing or
/// schema validation fails.
pub(super) fn validate_with_ref_validator(
    content: &str, validator: &Validator, code: &'static str, rule: &str,
) -> Result<()> {
    let instance: JsonValue = serde_saphyr::from_str(content)
        .map_err(|err| Error::validation_failed(code, rule, format!("parse failed: {err}")))?;
    let failures = validation_errors(validator, &instance);
    if failures.is_empty() {
        Ok(())
    } else {
        Err(Error::Validation {
            code: code.into(),
            detail: failures.join("; "),
        })
    }
}

pub(super) fn validation_failures(
    instance: &JsonValue, schema_source: &'static str, rule_id: &str, rule: &str,
) -> Vec<ValidationSummary> {
    validate_value_cached(instance, schema_source, rule_id, rule)
        .into_iter()
        .filter(|summary| summary.status == ValidationStatus::Fail)
        .collect()
}

pub(super) fn err_from_failures(code: &'static str, results: &[ValidationSummary]) -> Result<()> {
    if results.is_empty() {
        Ok(())
    } else {
        Err(Error::Validation {
            code: code.into(),
            detail: join_details(results),
        })
    }
}
