//! [`Diagnostic`] validation helpers.
//!
//! Two orthogonal checks:
//!
//! 1. **JSON Schema validation** — every wire field conforms to
//!    `schemas/diagnostics/diagnostic.schema.json` (kebab-case keys, closed
//!    enums, evidence `oneOf`, fingerprint pattern, etc.).
//! 2. **Evidence cap** — the serialized `evidence` object is bounded
//!    at 16 `KiB`. The cap covers the full evidence object (`kind` +
//!    payload), not individual fields.

use std::sync::LazyLock;

use serde_json::Value as JsonValue;
use specify_schema::{DIAGNOSTIC_JSON_SCHEMA, compile_schema};

use crate::diagnostic::Diagnostic;

/// 16 `KiB` cap on the serialized evidence object.
const EVIDENCE_MAX_BYTES: usize = 16 * 1024;

/// Diagnostic-schema validator, compiled once on first use.
///
/// A compile failure here means the embedded
/// `schemas/diagnostics/diagnostic.schema.json` is corrupt (a broken
/// binary), so the `expect` is genuinely unreachable in production and
/// mirrors the `LazyLock<Validator>` pattern the workflow layer uses
/// for embedded schema compilation.
static DIAGNOSTIC_VALIDATOR: LazyLock<jsonschema::Validator> = LazyLock::new(|| {
    compile_schema(DIAGNOSTIC_JSON_SCHEMA)
        .expect("embedded diagnostic schema compiles (corrupt binary otherwise)")
});

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
    validate_diagnostic_json(&value)
}

/// Validate a raw [`serde_json::Value`] against the embedded
/// `schemas/diagnostics/diagnostic.schema.json`.
///
/// # Errors
///
/// Returns [`DiagnosticError::Schema`] with a `; `-joined error list
/// when the instance fails validation.
pub fn validate_diagnostic_json(value: &JsonValue) -> Result<(), DiagnosticError> {
    let errors: Vec<String> = DIAGNOSTIC_VALIDATOR
        .iter_errors(value)
        .map(|err| format!("{}: {err}", err.instance_path()))
        .collect();
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
pub fn validate_evidence_size(diagnostic: &Diagnostic) -> Result<(), DiagnosticError> {
    let serialized = serde_json::to_string(&diagnostic.evidence)
        .map_err(|err| DiagnosticError::Serialize(err.to_string()))?;
    let actual = serialized.len();
    if actual > EVIDENCE_MAX_BYTES {
        Err(DiagnosticError::EvidenceTooLarge { actual })
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        DiagnosticError, EVIDENCE_MAX_BYTES, validate_diagnostic, validate_diagnostic_json,
        validate_evidence_size,
    };
    use crate::diagnostic::FindingEvidence;
    use crate::fingerprint::fingerprint;
    use crate::test_support::sample_diagnostic;

    fn sample_diagnostic_json() -> serde_json::Value {
        json!({
            "id": "FIND-0001",
            "rule-id": "UNI-014",
            "title": "Literal deployment URL in generated handler",
            "severity": "important",
            "source": "hybrid",
            "target-adapter": "omnia",
            "slice": "billing-invoice-export",
            "artifact": "code",
            "location": { "path": "crates/invoice_export/src/config.rs", "line": 18 },
            "evidence": {
                "kind": "snippet",
                "value": "const BASE_URL: &str = \"https://api.example.com\";"
            },
            "impact": "Generated code points every deployment at one endpoint.",
            "remediation": "Route the endpoint through Omnia configuration.",
            "confidence": "high",
            "fingerprint": "sha256:0000000000000000000000000000000000000000000000000000000000000000"
        })
    }

    #[test]
    fn rejects_empty_title() {
        let mut diagnostic = sample_diagnostic();
        diagnostic.title = String::new();
        diagnostic.fingerprint = fingerprint(&diagnostic);
        match validate_diagnostic(&diagnostic) {
            Err(DiagnosticError::Schema(detail)) => assert!(detail.contains("title")),
            other => panic!("expected Schema error, got {other:?}"),
        }
    }

    #[test]
    fn rejects_invalid_severity() {
        let mut value = sample_diagnostic_json();
        value["severity"] = json!("high");
        match validate_diagnostic_json(&value) {
            Err(DiagnosticError::Schema(detail)) => assert!(detail.contains("severity")),
            other => panic!("expected Schema error, got {other:?}"),
        }
    }

    /// A `review`-kind diagnostic is a legal wire shape.
    #[test]
    fn accepts_review_kind() {
        let mut value = sample_diagnostic_json();
        value["kind"] = json!("review");
        validate_diagnostic_json(&value).expect("review kind must validate");
    }

    #[test]
    fn evidence_size_rejects_oversize_snippet() {
        let mut diagnostic = sample_diagnostic();
        diagnostic.evidence = FindingEvidence::Snippet {
            value: "a".repeat(20 * 1024),
        };
        diagnostic.fingerprint = fingerprint(&diagnostic);
        match validate_evidence_size(&diagnostic) {
            Err(DiagnosticError::EvidenceTooLarge { actual }) => {
                assert!(actual > EVIDENCE_MAX_BYTES);
            }
            other => panic!("expected EvidenceTooLarge, got {other:?}"),
        }
    }

    #[test]
    fn rejects_invalid_rule_id() {
        let mut value = sample_diagnostic_json();
        value["rule-id"] = json!("OMNIA-1");
        match validate_diagnostic_json(&value) {
            Err(DiagnosticError::Schema(detail)) => assert!(detail.contains("rule-id")),
            other => panic!("expected Schema error, got {other:?}"),
        }
    }

    #[test]
    fn rejects_snippet_with_extra_sha256() {
        let mut value = sample_diagnostic_json();
        value["evidence"] = json!({
            "kind": "snippet",
            "value": "let x = 1;",
            "sha256": "a".repeat(64),
        });
        match validate_diagnostic_json(&value) {
            Err(DiagnosticError::Schema(detail)) => assert!(detail.contains("evidence")),
            other => panic!("expected Schema error, got {other:?}"),
        }
    }
}
