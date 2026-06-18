//! Shared CLI output format and the single [`emit`] entry point used by
//! every `specify` surface, plus the shared lint output tail:
//! [`run_lint`] is the one kernel both lint handlers call —
//! [`emit_lint_report`] renders one envelope and the internal
//! `finish_lint` turns the outcome into the handler's terminal
//! `Result<()>` so both lint surfaces differ only in pipeline config,
//! not output/exit plumbing.

use std::io::Write;
use std::time::Instant;

use clap::ValueEnum;
use jiff::Timestamp;
use serde::Serialize;
use specify_diagnostics::{
    DiagnosticReport, DiagnosticReportVersion, DiagnosticSummary, Format as DiagnosticsFormat,
    RenderError, render,
};
use specify_error::{Error, Result};
use specify_standards::ResolveInputs;
use specify_standards::lint::diagnostics::map_render_error;
use specify_standards::lint::ignore::{blocking_findings_present, deny_blocking_findings};
use specify_standards::lint::runner::{PipelineConfig, RunOutcome, run as run_pipeline};
use specify_workflow::config::Layout;
use specify_workflow::journal::{self, LintScope};

/// Structured (`json`) or human (`text`) CLI output.
#[derive(Copy, Clone, Debug, ValueEnum, PartialEq, Eq)]
pub enum Format {
    /// Human-readable lines on stdout/stderr.
    Text,
    /// Pretty-printed JSON envelopes for skill/CI consumption.
    Json,
}

/// Emit `payload` through `writer` in the requested format. JSON
/// serialises the body directly via `serde_json::to_writer_pretty`;
/// Text delegates to `render_text`. The single signature covers
/// both success (stdout) and failure (stderr) — there is one entry
/// point for all structured output. Callers construct the locked
/// writer at the boundary so the sink choice is visible at the
/// call site.
///
/// # Errors
///
/// Propagates the underlying serialization or I/O error.
pub fn emit<T: Serialize>(
    writer: &mut dyn Write, format: Format, payload: &T,
    render_text: impl FnOnce(&mut dyn Write, &T) -> std::io::Result<()>,
) -> Result<(), Error> {
    match format {
        Format::Json => {
            serde_json::to_writer_pretty(&mut *writer, payload).map_err(|err| Error::Diag {
                code: "json-serialize-failed",
                detail: format!("failed to serialize JSON response: {err}"),
            })?;
            writeln!(writer).map_err(Error::Io)
        }
        Format::Text => render_text(writer, payload).map_err(Error::Io),
    }
}

/// The shared lint-output tail for the `specify lint` and `specify lint framework`
/// handlers. Carries the already-composed
/// [`DiagnosticReport`] plus everything [`emit_diagnostic_report`] needs
/// to render it, journal the run, and decide blocking status in one
/// place so neither handler re-implements the sequence.
pub struct LintEmit<'a> {
    /// Wire format for the rendered envelope.
    pub format: DiagnosticsFormat,
    /// The composed report to render and journal.
    pub report: &'a DiagnosticReport,
    /// Project (or framework-root) layout the journal event lands under.
    pub layout: Layout<'a>,
    /// Dispatcher-injected event timestamp for the `lint-completed`
    /// event (architecture §Time injection); the handler reads the clock.
    pub now: Timestamp,
    /// Scope facets recorded on the `lint-completed` event.
    pub scope: LintScope,
    /// Whether to append a `lint-completed` journal event. `specify lint
    /// project` journals (runtime observability); `specify lint framework`
    /// does not — framework self-lint is a development surface and the
    /// `lint-completed` contract is scoped to `lint project` (DECISIONS.md
    /// §"Journal event names").
    pub journal: bool,
    /// Command label prefixed onto best-effort journal failures.
    pub command_label: &'static str,
    /// Wall-clock duration of the scan in milliseconds.
    pub elapsed_ms: u128,
    /// Append an extra newline after the (already newline-terminated)
    /// body. `specify lint` does (`println!`); `specify lint framework`
    /// does not (`print!`) — preserved here so neither surface's stdout
    /// shape shifts under the unification.
    pub trailing_newline: bool,
}

/// Render the lint envelope to stdout, append at most one
/// `lint-completed` journal event (only when `emit.journal` is set),
/// and report whether any blocking (`critical | important`) finding is
/// present.
///
/// Shared by both lint handlers so the render → print → journal →
/// blocking-decision sequence lives in one place. Callers map the
/// returned [`RenderError`] and the blocking flag onto their own
/// surface conventions (both lint surfaces map it onto `Result`).
///
/// # Errors
///
/// Propagates the [`RenderError`] from envelope rendering; the journal
/// emit is best-effort and never surfaces as an error.
pub fn emit_diagnostic_report(emit: LintEmit<'_>) -> Result<bool, RenderError> {
    let rendered = render(emit.format, emit.report)?;
    if emit.trailing_newline {
        println!("{rendered}");
    } else {
        print!("{rendered}");
    }
    let blocking = blocking_findings_present(&emit.report.findings);
    let exit_code: i32 = if blocking { 2 } else { 0 };
    if emit.journal {
        journal::emit_lint_completed(
            emit.layout,
            emit.now,
            emit.scope,
            &emit.report.findings,
            emit.elapsed_ms,
            exit_code,
            emit.command_label,
        );
    }
    Ok(blocking)
}

/// Everything [`emit_lint_report`] needs to run one lint pipeline and
/// render its envelope. The two lint surfaces (`specify lint project`,
/// `specify lint framework`) differ only in the inputs and config they assemble
/// here — the render → journal → blocking-decision tail is shared.
pub struct LintRun<'a> {
    /// Resolver inputs (project dir, rules root, adapters, filters).
    pub inputs: &'a ResolveInputs<'a>,
    /// Pipeline config (profile, tool runner, degradation).
    pub config: &'a PipelineConfig<'a>,
    /// Wire format for the rendered envelope and the abort fallback.
    pub format: DiagnosticsFormat,
    /// Project (or framework-root) layout the journal event lands under.
    pub layout: Layout<'a>,
    /// Dispatcher-injected event timestamp for the `lint-completed`
    /// event (architecture §Time injection); the handler reads the clock.
    pub now: Timestamp,
    /// Scope facets recorded on the `lint-completed` event.
    pub scope: LintScope,
    /// Whether to append a `lint-completed` journal event. See
    /// [`LintEmit::journal`].
    pub journal: bool,
    /// Command label prefixed onto best-effort journal failures.
    pub command_label: &'static str,
    /// Scan clock started at handler entry; the elapsed span lands on
    /// the journal event.
    pub started_at: Instant,
    /// Append an extra newline after the body (`specify lint` does,
    /// `specify lint framework` does not). See [`LintEmit::trailing_newline`].
    pub trailing_newline: bool,
}

/// Run the lint pipeline and render its envelope on stdout.
///
/// Returns the composed [`DiagnosticReport`] so the caller can run the
/// blocking-decision gate, or `None` for the `--dump-model`
/// short-circuit (whose model body has already reached stdout). Any
/// `Err` is a pre-emit / emit-time abort — the real report never
/// reached stdout — which [`finish_lint`] turns into the empty
/// fallback envelope.
///
/// # Errors
///
/// Propagates pipeline resolution / indexing / evaluation errors and
/// the [`map_render_error`]-mapped envelope render failure.
pub fn emit_lint_report(run: LintRun<'_>) -> Result<Option<DiagnosticReport>> {
    match run_pipeline(run.inputs, run.config)? {
        RunOutcome::DumpedModel => Ok(None),
        RunOutcome::Report(report) => {
            emit_diagnostic_report(LintEmit {
                format: run.format,
                report: &report,
                layout: run.layout,
                now: run.now,
                scope: run.scope,
                journal: run.journal,
                command_label: run.command_label,
                elapsed_ms: run.started_at.elapsed().as_millis(),
                trailing_newline: run.trailing_newline,
            })
            .map_err(map_render_error)?;
            Ok(Some(report))
        }
    }
}

/// Drive one lint surface end to end: run the caller's pipeline
/// assembly + emit closure, then collapse its outcome into the
/// terminal `Result<()>`. This is the single kernel both lint handlers
/// (`specify lint project`, `specify lint framework`) call, so the build → emit →
/// finish → blocking-gate sequence lives in one place; the handlers
/// differ only in the `PipelineConfig` their `build` closure assembles.
///
/// `build` owns every pre-emit `?` abort (scope composition, tool
/// runner construction, framework-root load) plus the
/// [`emit_lint_report`] call; any `Err` it returns is routed through
/// the JSON fallback below so structured consumers keep a stable
/// envelope shape even when the run aborts before emit.
///
/// # Errors
///
/// Propagates the closure's abort error or the blocking-finding
/// `Error::Validation` from [`deny_blocking_findings`].
pub fn run_lint(
    format: DiagnosticsFormat, build: impl FnOnce() -> Result<Option<DiagnosticReport>>,
) -> Result<()> {
    finish_lint(format, build())
}

/// Collapse a lint run's [`emit_lint_report`] outcome into the
/// handler's terminal `Result<()>`. Driven only by [`run_lint`] so the
/// failure-render seam lives in one place:
///
/// - `Ok(Some(report))` — the envelope is already on stdout; gate the
///   exit on blocking findings via [`deny_blocking_findings`].
/// - `Ok(None)` — `--dump-model` already emitted its body; succeed.
/// - `Err(err)` — the run aborted before emitting its real report;
///   render the empty fallback envelope on stdout (JSON only) so CI
///   consumers keep a stable shape, then propagate the error. The
///   matching stderr `error: …` line is the dispatcher's
///   `output::report`, so the two sinks compose without double-print.
fn finish_lint(format: DiagnosticsFormat, built: Result<Option<DiagnosticReport>>) -> Result<()> {
    match built {
        Ok(Some(report)) => deny_blocking_findings(&report),
        Ok(None) => Ok(()),
        Err(err) => {
            emit_empty_report_on_abort(format);
            Err(err)
        }
    }
}

/// Render an empty all-zero [`DiagnosticReport`] on **stdout** when a
/// lint run aborts before composing its real report, but only for JSON
/// output — so structured CI consumers always receive a stable
/// envelope shape. A no-op for the human formatters (`pretty | github
/// | compact`), whose only failure signal is the stderr `error: …`
/// line.
///
/// Owns the stdout side of the abort path only; the stderr line is the
/// dispatcher's `output::report`.
fn emit_empty_report_on_abort(format: DiagnosticsFormat) {
    if !matches!(format, DiagnosticsFormat::Json) {
        return;
    }
    let report = DiagnosticReport {
        version: DiagnosticReportVersion,
        summary: DiagnosticSummary::default(),
        findings: Vec::new(),
    };
    // The empty envelope is schema-valid by construction, so a render
    // error is unreachable; on the impossible path leave stdout empty
    // rather than emit a malformed body.
    if let Ok(rendered) = render(DiagnosticsFormat::Json, &report) {
        print!("{rendered}");
    }
}
