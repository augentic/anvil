//! Specify neutral diagnostic substrate, plus the [`digest`] SHA-256
//! helpers and the digest-keyed [`cache`] path plumbing.
//!
//! This crate owns the [`Diagnostic`] currency shared by both Specify
//! check surfaces — the advisory review surface and the
//! workflow-gating `validate` surface — together with the fingerprint
//! algorithm. The two surfaces stay conceptually distinct (they differ
//! in gate policy, not in currency); naming the substrate neutrally
//! lets every producer mint diagnostics without importing the other
//! surface's code: `artifacts` (which holds the `validate` registry)
//! depends on this leaf.
//!
//! Artifact shapes are owned by their Rust types (serde is the load
//! gate); the judgment answer schemas the model host consumes are
//! generated from those types by `project::answers` / `slice::answers`.
//! This crate carries no JSON Schema machinery and no workflow
//! lifecycle types, so every higher layer can build on it without
//! inheriting a heavier graph.

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
