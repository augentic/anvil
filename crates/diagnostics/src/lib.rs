//! Emery neutral diagnostic substrate, plus the [`digest`] SHA-256
//! helpers and the digest-keyed [`cache`] path plumbing.
//!
//! The [`Diagnostic`] currency is shared by every check surface.

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
