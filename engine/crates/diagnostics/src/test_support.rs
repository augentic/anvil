//! Shared diagnostic fixtures for the crate's unit tests.

use crate::diagnostic::{
    Artifact, Confidence, Diagnostic, DiagnosticKind, DiagnosticSource, FindingEvidence,
    FindingLocation, Severity,
};

/// Minimal diagnostic with the given `id` and `severity`; every other
/// field is a fixed placeholder. Used by the report-summary tests.
pub fn diagnostic(id: &str, severity: Severity) -> Diagnostic {
    Diagnostic {
        id: id.into(),
        rule_id: None,
        related_rule_ids: None,
        title: "t".into(),
        severity,
        source: DiagnosticSource::Deterministic,
        kind: DiagnosticKind::Violation,
        target_adapter: None,
        source_adapter: None,
        slice: None,
        change: None,
        artifact: Artifact::Code,
        location: None,
        evidence: FindingEvidence::Snippet { value: "x".into() },
        impact: "i".into(),
        remediation: "r".into(),
        confidence: Some(Confidence::High),
        fingerprint: "sha256:0000000000000000000000000000000000000000000000000000000000000000"
            .into(),
        status: None,
        disposition: None,
    }
}

/// Fully-populated diagnostic template used by the fingerprint and
/// validate tests. Callers mutate fields and recompute the
/// fingerprint to assert which dimensions enter the hash.
pub fn sample_diagnostic() -> Diagnostic {
    Diagnostic {
        id: "FIND-0001".into(),
        rule_id: Some("UNI-014".into()),
        related_rule_ids: None,
        title: "Literal deployment URL in generated handler".into(),
        severity: Severity::Important,
        source: DiagnosticSource::Hybrid,
        kind: DiagnosticKind::Violation,
        target_adapter: Some("omnia".into()),
        source_adapter: None,
        slice: Some("billing-invoice-export".into()),
        change: None,
        artifact: Artifact::Code,
        location: Some(FindingLocation {
            path: "crates/invoice_export/src/config.rs".into(),
            line: Some(18),
            column: Some(5),
            end_line: None,
            end_column: None,
        }),
        evidence: FindingEvidence::Snippet {
            value: "const BASE_URL: &str = \"https://api.example.com\";".into(),
        },
        impact: "Generated code will point every deployment at the same external endpoint.".into(),
        remediation:
            "Read the endpoint from Omnia configuration and add a required config key to the design."
                .into(),
        confidence: Some(Confidence::High),
        fingerprint: String::new(),
        status: None,
        disposition: None,
    }
}
