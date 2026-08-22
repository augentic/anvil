//! Neutral diagnostics, hashing, and cache metadata.

pub mod cache;
pub mod diagnostic;
pub mod digest;
pub mod fingerprint;

pub use diagnostic::{
    Artifact, Confidence, Diagnostic, DiagnosticKind, DiagnosticReport, DiagnosticReportVersion,
    DiagnosticSource, DiagnosticSummary, FindingEvidence, FindingLocation, Severity, has_blocking,
    is_blocking, renumber,
};
pub use fingerprint::{canonical_json, fingerprint, verify_fingerprint};
