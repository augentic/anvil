//! The RFC-90 D2 phase-report acceptance gate: [`accept`] structurally
//! gates every returned report; a rejection terminates the attempt
//! with an engine-authored diagnostic via [`engine_finding`].

use diagnostics::{Artifact, Diagnostic, DiagnosticKind, DiagnosticSource, Severity, is_blocking};
use error::{Error, Result};
use project::seam::wire::{PhaseOutcome, PhaseReport, PhaseSource};

/// The closed set of operations whose reports pass this gate — the
/// build-loop subset of the target axis (`guidance` and `merge` never
/// return a phase report).
#[derive(Debug, Clone, Copy, PartialEq, Eq, strum::Display)]
#[strum(serialize_all = "kebab-case")]
pub enum PhaseOperation {
    /// Generation: preparation, writers, capture replay.
    Build,
    /// One check pass over the lent workspace.
    Verify,
    /// One findings-directed repair pass.
    Repair,
    /// One engineering-standards review pass.
    Review,
}

/// Maximum accepted `next_continuation` payload in bytes (1 MiB) —
/// an engine constant (RFC-90 D2).
pub const CONTINUATION_LIMIT: usize = 1024 * 1024;

/// Accept or reject one returned phase report (RFC-90 D2).
///
/// # Errors
///
/// Returns [`Error::Diag`] keyed on the `target-phase-*` family:
///
/// - `target-phase-source-tool` — report source `tool` is rejected
///   until a trusted host-tool execution seam exists (RFC-97);
/// - `target-phase-source-incoherent` — the report-level source does
///   not cover its finding sources: `deterministic` admits only
///   deterministic findings, `model-assisted` only model-assisted
///   ones; `hybrid` covers any mix (including none — the source is an
///   assurance claim that holds even when `findings` is empty);
///   finding sources `tool` and `human` never cross this seam;
/// - `target-phase-output-declaration` — a non-`build` operation
///   declared outputs, a UI surface, or a coverage claim (`build`
///   alone owns those candidate values);
/// - `target-phase-not-applicable-dirty` — a `not-applicable` outcome
///   carried a blocking finding or a `written` entry;
/// - `target-phase-write-escape` — a `written` path is empty,
///   absolute, backslashed, or contains a `..` segment;
/// - `target-phase-location-escape` — a finding location path breaks
///   the same rules;
/// - `target-phase-verify-continuation` — `verify` returned a
///   continuation (verify cannot mutate it);
/// - `target-phase-continuation-oversized` — the continuation exceeds
///   [`CONTINUATION_LIMIT`] bytes.
pub fn accept(operation: PhaseOperation, report: &PhaseReport) -> Result<()> {
    if report.source == PhaseSource::Tool {
        return Err(Error::Diag {
            code: "target-phase-source-tool",
            detail: format!(
                "the `{operation}` report claims source `tool`, which is rejected until a \
                 trusted host-tool execution seam exists (RFC-97)"
            ),
        });
    }
    check_source_coherence(operation, report)?;
    if operation != PhaseOperation::Build
        && (!report.outputs.is_empty() || report.ui_surface.is_some() || !report.covered.is_empty())
    {
        return Err(Error::Diag {
            code: "target-phase-output-declaration",
            detail: format!(
                "the `{operation}` report declares outputs, a ui-surface, or a coverage claim; \
                 only `build` owns output declaration, UI-surface classification, and \
                 requirement coverage"
            ),
        });
    }
    if report.outcome == PhaseOutcome::NotApplicable
        && (report.findings.iter().any(is_blocking) || !report.written.is_empty())
    {
        return Err(Error::Diag {
            code: "target-phase-not-applicable-dirty",
            detail: format!(
                "the `{operation}` report is not-applicable but carries a blocking finding or a \
                 written entry; a not-applicable report must be clean"
            ),
        });
    }
    for write in &report.written {
        if malformed_relative_path(&write.path) {
            return Err(Error::Diag {
                code: "target-phase-write-escape",
                detail: format!(
                    "the `{operation}` report's written path `{}` is empty, absolute, \
                     backslashed, or contains `..`",
                    write.path
                ),
            });
        }
    }
    for finding in &report.findings {
        if let Some(location) = &finding.location
            && malformed_relative_path(&location.path)
        {
            return Err(Error::Diag {
                code: "target-phase-location-escape",
                detail: format!(
                    "the `{operation}` report's finding location `{}` is empty, absolute, \
                     backslashed, or contains `..`",
                    location.path
                ),
            });
        }
    }
    if operation == PhaseOperation::Verify && report.next_continuation.is_some() {
        return Err(Error::Diag {
            code: "target-phase-verify-continuation",
            detail: "the `verify` report returned a continuation; verify cannot mutate the \
                     attempt's continuation state"
                .to_string(),
        });
    }
    if let Some(continuation) = &report.next_continuation
        && continuation.len() > CONTINUATION_LIMIT
    {
        return Err(Error::Diag {
            code: "target-phase-continuation-oversized",
            detail: format!(
                "the `{operation}` report's continuation is {} bytes; the engine rejects \
                 payloads over {CONTINUATION_LIMIT} bytes before persistence",
                continuation.len()
            ),
        });
    }
    Ok(())
}

/// The D2 source/finding coherence rule.
///
/// Allowed finding-source sets per report source: `deterministic` →
/// `{}` or `{deterministic}`; `model-assisted` → `{}` or
/// `{model-assisted}`; `hybrid` → any subset of `{deterministic,
/// model-assisted, hybrid}` including the empty set — the source is
/// an assurance claim stating how the pass was produced even when
/// `findings` is empty (RFC-90 D2). Finding sources `tool` and
/// `human` always reject.
fn check_source_coherence(operation: PhaseOperation, report: &PhaseReport) -> Result<()> {
    let mut deterministic = false;
    let mut model_assisted = false;
    let mut hybrid = false;
    let mut untrusted = false;
    for finding in &report.findings {
        match finding.source {
            DiagnosticSource::Deterministic => deterministic = true,
            DiagnosticSource::ModelAssisted => model_assisted = true,
            DiagnosticSource::Hybrid => hybrid = true,
            DiagnosticSource::Human | DiagnosticSource::Tool => untrusted = true,
        }
    }
    let coherent = !untrusted
        && match report.source {
            PhaseSource::Deterministic => !model_assisted && !hybrid,
            PhaseSource::ModelAssisted => !deterministic && !hybrid,
            PhaseSource::Hybrid => true,
            // Rejected before coherence by the `target-phase-source-tool` gate.
            PhaseSource::Tool => false,
        };
    if coherent {
        return Ok(());
    }
    Err(Error::Diag {
        code: "target-phase-source-incoherent",
        detail: format!(
            "the `{operation}` report's source `{}` does not cover its finding sources: a \
             deterministic report admits only deterministic findings, a model-assisted report \
             only model-assisted findings, and finding sources `tool` / `human` never cross \
             this seam",
            report.source
        ),
    })
}

/// `true` when `path` is not a well-formed relative '/'-separated
/// path: empty, absolute (leading `/` or a Windows drive), containing
/// a backslash, or containing a `..` segment.
pub(crate) fn malformed_relative_path(path: &str) -> bool {
    path.is_empty()
        || path.starts_with('/')
        || path.contains('\\')
        || windows_drive(path)
        || path.split('/').any(|segment| segment == "..")
}

/// `true` when `path` opens with a Windows drive prefix (`C:`).
fn windows_drive(path: &str) -> bool {
    let bytes = path.as_bytes();
    bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
}

/// An engine-authored blocking diagnostic for a terminal gate
/// failure (RFC-90 D1).
///
/// Deterministic, `important`, `violation`, `code` — the shape the
/// phase machine folds into a failed terminal report when a dispatch
/// error, invalid report, or engine gate terminates the attempt.
#[must_use]
pub fn engine_finding(code: &str, title: &str, detail: &str) -> Diagnostic {
    Diagnostic::finding(
        code,
        title,
        detail,
        Severity::Important,
        DiagnosticKind::Violation,
        DiagnosticSource::Deterministic,
        Artifact::Code,
        None,
    )
}
