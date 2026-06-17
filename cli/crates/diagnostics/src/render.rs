//! Diagnostic formatter umbrella.
//!
//! Ships the four formatters of the closed set
//! ([`Format::Json`], [`Format::Pretty`], [`Format::Github`],
//! [`Format::Compact`]). Rendering lives in this crate so every
//! surface that emits a [`DiagnosticReport`] (`specify lint`, `specify
//! lint framework`, the slice/plan validate gates) cannot drift.
//!
//! Only the [`Format::Json`] formatter validates against
//! [`specify_schema::DIAGNOSTIC_REPORT_JSON_SCHEMA`] before emit; the other
//! three are presentation layers over the same in-memory
//! [`DiagnosticReport`].

pub mod compact;
pub mod github;
pub mod json;
pub mod pretty;

use thiserror::Error;

use crate::diagnostic::DiagnosticReport;

/// Closed formatter discriminant.
///
/// Kept clap-free at the substrate boundary; CLI surfaces adapt this
/// enum to their own `clap::ValueEnum`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Format {
    /// Diagnostic-report wire envelope; schema-validated before emit.
    Json,
    /// Terminal output with severity colour and source location.
    Pretty,
    /// GitHub Actions workflow-annotation lines.
    Github,
    /// Tab-separated one-line-per-diagnostic shape.
    Compact,
}

/// Closed render error.
///
/// Only [`Format::Json`] validates against the embedded schema before
/// emit, so it is the only formatter that can surface
/// [`RenderError::JsonSchemaValidation`].
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum RenderError {
    /// JSON envelope failed [`specify_schema::DIAGNOSTIC_REPORT_JSON_SCHEMA`].
    #[error("diagnostic-report envelope failed schema validation: {detail}")]
    JsonSchemaValidation {
        /// Joined `; `-separated validator error list.
        detail: String,
    },
    /// `serde_json::to_string_pretty` failed.
    #[error("diagnostic-report JSON serialisation failed: {0}")]
    JsonSerialise(#[from] serde_json::Error),
}

/// Render `report` using the requested `format`.
///
/// # Errors
///
/// - [`RenderError::JsonSchemaValidation`] when `format` is
///   [`Format::Json`] and the serialised envelope fails the v1 schema.
/// - [`RenderError::JsonSerialise`] when JSON serialisation itself
///   fails (unreachable for a typed [`DiagnosticReport`]).
pub fn render(format: Format, report: &DiagnosticReport) -> Result<String, RenderError> {
    match format {
        Format::Json => json::render(report),
        Format::Pretty => pretty::render(report),
        Format::Github => github::render(report),
        Format::Compact => compact::render(report),
    }
}
