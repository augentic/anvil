//! Specify neutral diagnostic substrate.
//!
//! This leaf crate owns the [`Diagnostic`] currency shared by both
//! Specify check surfaces — the advisory `lint` surface and the
//! workflow-gating `validate` surface — together with the
//! fingerprint algorithm, the validators, and the four renderers.
//!
//! The two surfaces stay conceptually distinct (they differ in gate
//! policy, not in currency). Naming the substrate neutrally — rather
//! than after `lint` — lets `validate` produce diagnostics without
//! depending on anything named `lint`: `specify-model` (which holds the
//! `validate` registry) depends on this leaf, and `specify-standards`
//! re-exports it for the advisory surface.
//!
//! Dependency posture: depends only on `specify-error` and
//! `specify-schema` (plus `serde`/`serde_json`/`jsonschema`). The
//! SHA-256 fingerprint digest comes through `specify_schema::digest`.
//! It carries no workflow lifecycle types and no `WorkspaceModel`, so
//! every higher layer can build on it without inheriting a heavier
//! graph.

pub mod diagnostic;
pub mod fingerprint;
pub mod render;
pub mod validate;

#[cfg(test)]
mod test_support;

pub use diagnostic::{
    Artifact, Confidence, Diagnostic, DiagnosticKind, DiagnosticReport, DiagnosticReportVersion,
    DiagnosticSource, DiagnosticSummary, DirectiveDisposition, DispositionSource,
    FindingDisposition, FindingEvidence, FindingLocation, FindingStatus, Severity, blocking,
    blocking_present, count_status, renumber,
};
pub use fingerprint::{canonical_json, fingerprint, verify_fingerprint};
pub use render::{Format, RenderError, render};
pub use validate::{
    DiagnosticError, validate_diagnostic, validate_diagnostic_json, validate_evidence_size,
};
