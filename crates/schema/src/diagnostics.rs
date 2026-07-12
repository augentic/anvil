//! Specify neutral diagnostic substrate.
//!
//! This module owns the [`Diagnostic`] currency shared by both
//! Specify check surfaces — the advisory review surface and the
//! workflow-gating `validate` surface — together with the
//! fingerprint algorithm and the validators.
//!
//! The two surfaces stay conceptually distinct (they differ in gate
//! policy, not in currency). Naming the substrate neutrally — rather
//! than after either surface — lets every producer mint diagnostics
//! without importing the other surface's code: `artifacts` (which
//! holds the `validate` registry) depends on this leaf.
//!
//! Lives on the `schema` leaf beside the embedded diagnostic JSON
//! Schemas it validates against and the `digest` module that backs the
//! SHA-256 fingerprint. It carries no workflow lifecycle types, so
//! every higher layer can build on it without inheriting a heavier
//! graph.

pub mod diagnostic;
pub mod fingerprint;
pub mod validate;

pub use diagnostic::{
    Artifact, Confidence, Diagnostic, DiagnosticKind, DiagnosticReport, DiagnosticReportVersion,
    DiagnosticSource, DiagnosticSummary, FindingEvidence, FindingLocation, Severity, has_blocking,
    is_blocking, renumber,
};
pub use fingerprint::{canonical_json, fingerprint, verify_fingerprint};
pub use validate::{DiagnosticError, validate_diagnostic};
