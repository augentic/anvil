//! [`Diagnostic`] validation helpers.
//!
//! Checks the wire shape and enforces the 16 `KiB` evidence cap.

use serde_json::Value as JsonValue;

use crate::diagnostics::diagnostic::Diagnostic;
use crate::{DIAGNOSTIC_JSON_SCHEMA, cached_validator, validation_errors};

/// 16 `KiB` cap on the serialized evidence object.
const EVIDENCE_MAX_BYTES: usize = 16 * 1024;

/// Closed failure mode for the diagnostic validators.
#[derive(Debug, thiserror::Error)]
pub enum DiagnosticError {
    /// JSON-schema validation failed. The string carries every
    /// JSON-pointer + reason pair joined by `; `.
    #[error("diagnostic schema validation failed: {0}")]
    Schema(String),
    /// Serialized evidence object exceeds the 16 `KiB` cap.
    #[error("diagnostic evidence exceeds 16 KiB cap (got {actual} bytes)")]
    EvidenceTooLarge {
        /// Byte length of the UTF-8 serialized evidence object.
        actual: usize,
    },
    /// Diagnostic could not be serialized to JSON.
    #[error("diagnostic JSON serialization failed: {0}")]
    Serialize(String),
}

/// Validate a typed [`Diagnostic`] against the embedded
/// `schemas/diagnostics/diagnostic.schema.json`.
///
/// # Errors
///
/// - [`DiagnosticError::Serialize`] if the typed diagnostic cannot be
///   serialized (unreachable for the derived `Serialize` impl).
/// - [`DiagnosticError::Schema`] when the wire shape violates the
///   embedded schema.
pub fn validate_diagnostic(diagnostic: &Diagnostic) -> Result<(), DiagnosticError> {
    let value = serde_json::to_value(diagnostic)
        .map_err(|err| DiagnosticError::Serialize(err.to_string()))?;
    validate_diagnostic_json(&value)?;
    validate_evidence_size(diagnostic)
}

/// Validate a raw [`serde_json::Value`] against the embedded
/// `schemas/diagnostics/diagnostic.schema.json`.
///
/// # Errors
///
/// Returns [`DiagnosticError::Schema`] with a `; `-joined error list
/// when the instance fails validation.
fn validate_diagnostic_json(value: &JsonValue) -> Result<(), DiagnosticError> {
    let validator = cached_validator(DIAGNOSTIC_JSON_SCHEMA)
        .map_err(|err| DiagnosticError::Schema(err.to_string()))?;
    let errors = validation_errors(&validator, value);
    if errors.is_empty() { Ok(()) } else { Err(DiagnosticError::Schema(errors.join("; "))) }
}

/// Enforce the 16 `KiB` serialized evidence cap.
///
/// # Errors
///
/// - [`DiagnosticError::Serialize`] if the evidence cannot be
///   serialized (unreachable for the derived `Serialize` impl).
/// - [`DiagnosticError::EvidenceTooLarge`] when the serialized form
///   exceeds 16 `KiB`.
fn validate_evidence_size(diagnostic: &Diagnostic) -> Result<(), DiagnosticError> {
    let serialized = serde_json::to_string(&diagnostic.evidence)
        .map_err(|err| DiagnosticError::Serialize(err.to_string()))?;
    let actual = serialized.len();
    if actual > EVIDENCE_MAX_BYTES {
        Err(DiagnosticError::EvidenceTooLarge { actual })
    } else {
        Ok(())
    }
}
