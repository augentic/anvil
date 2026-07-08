//! `specify lint framework` handler — runs the declarative
//! deterministic-hint interpreter into a single [`DiagnosticReport`]
//! envelope.
//!
//! The shared pipeline lives in [`specify_standards::lint::runner`];
//! this handler is thin and obeys the standard `Result<()>` handler
//! contract, assembling only the framework-surface config:
//!
//! 1. Resolve the framework root (every `rules.*` check runs through
//!    the in-process `rules` checker, so no imperative producer is
//!    wired in).
//! 2. Configure the runner for the framework surface
//!    ([`ScanProfile::Framework`], [`NoopToolRunner`] — framework
//!    `kind: tool` rules resolve in-process, `include_core: true`).
//!    Codex resolution is fatal: a duplicate rule id or unresolvable
//!    rules tree aborts the run rather than silently passing.
//! 3. Render the envelope, decide the blocking exit, and own the JSON
//!    fallback on abort. Framework self-lint is a development surface
//!    and never journals (DECISIONS.md §"Journal event names").

use std::path::{Path, PathBuf};

use specify_diagnostics::{
    DiagnosticReport, DiagnosticReportVersion, DiagnosticSummary, Format as DiagnosticsFormat,
    render,
};
use specify_error::{Error, Result};
use specify_standards::ResolveInputs;
use specify_standards::lint::ScanProfile;
use specify_standards::lint::diagnostics::map_render_error;
use specify_standards::lint::eval::NoopToolRunner;
use specify_standards::lint::ignore::deny_blocking_findings;
use specify_standards::lint::runner::{PipelineConfig, RunOutcome, run as run_pipeline};

use crate::commands::lint::cli::{FrameworkArgs, LintFormat};
use crate::output::Format;

/// Handler entry point dispatched from the guest router.
///
/// Returns `Result<()>` like every handler; the dispatcher maps the
/// terminal error through the shared `Exit::from(&Error)` table.
/// Always leaves a stable envelope on stdout for JSON output — the
/// real report on success, an empty all-zero envelope when the run
/// aborts before emit.
///
/// # Errors
///
/// Propagates the framework-root load error, any pipeline / render
/// abort, and the blocking-finding `Error::Validation` from
/// [`deny_blocking_findings`].
pub fn run(format: Format, action: &FrameworkArgs) -> Result<()> {
    let diagnostics_format = pick_format(format, action.output_format);
    match build_report(action, diagnostics_format) {
        Ok(Some(report)) => deny_blocking_findings(&report),
        Ok(None) => Ok(()),
        Err(err) => {
            emit_empty_report_on_abort(diagnostics_format);
            Err(err)
        }
    }
}

/// Assemble the framework-surface inputs and config, run the shared
/// pipeline, and render the envelope on stdout. Returns the composed
/// report for the blocking-decision gate, or `None` for the
/// `--dump-model` short-circuit (whose model body has already reached
/// stdout). Any `Err` is a pre-emit abort the caller turns into the
/// JSON fallback envelope.
fn build_report(
    action: &FrameworkArgs, format: DiagnosticsFormat,
) -> Result<Option<DiagnosticReport>> {
    let project_dir = framework_root(&action.framework_root)?;

    let inputs = ResolveInputs {
        project_dir: &project_dir,
        rules_root: Some(&project_dir),
        target_adapter: &action.target,
        source_adapters: &action.sources,
        artifact_paths: &action.artifacts,
        languages: &action.languages,
        include_deprecated: false,
        include_unmatched: false,
        include_core: true,
    };

    // Every `rules.*` check (namespace ownership, duplicate id) runs
    // through the in-process `rules` checker via `kind: tool`,
    // resolved in-process by the declarative pass — the runner is
    // never consulted for framework checkers.
    let rule_filter_slice: Vec<&str> = action.rules.iter().map(String::as_str).collect();
    let tool_runner = NoopToolRunner;
    let config = PipelineConfig {
        profile: ScanProfile::Framework,
        dump_model: action.dump_model,
        apply_ignore_directives: true,
        rule_filter: &rule_filter_slice,
        tool_runner: &tool_runner,
    };

    match run_pipeline(&inputs, &config)? {
        RunOutcome::DumpedModel => Ok(None),
        RunOutcome::Report(report) => {
            let rendered = render(format, &report).map_err(map_render_error)?;
            print!("{rendered}");
            Ok(Some(report))
        }
    }
}

/// Resolve the framework root after a structural sanity check (at
/// least one flattened adapter tree — `codex/`, `sources/`, or
/// `targets/` — with or without `plugins/`). Canonicalisation is
/// best-effort: the guest's `"."` preopen may not support
/// `canonicalize`, and every downstream path stays anchored at the
/// (possibly relative) root either way.
fn framework_root(root: &Path) -> Result<PathBuf> {
    if !["codex", "sources", "targets"].iter().any(|dir| root.join(dir).is_dir()) {
        return Err(Error::Diag {
            code: "framework-root",
            detail: format!("not a framework root: {}", root.display()),
        });
    }
    Ok(root.canonicalize().unwrap_or_else(|_| root.to_path_buf()))
}

/// Resolve the diagnostics format for the success body. Per-subcommand
/// `--output-format` wins; otherwise mirror the global `--format`
/// flag so `specify lint framework --format json` still emits the wire
/// envelope.
fn pick_format(global: Format, output_format: Option<LintFormat>) -> DiagnosticsFormat {
    if let Some(value) = output_format {
        return value.into();
    }
    match global {
        Format::Json => DiagnosticsFormat::Json,
        Format::Text => DiagnosticsFormat::Pretty,
    }
}

/// Render an empty all-zero [`DiagnosticReport`] on **stdout** when a
/// lint run aborts before composing its real report, but only for JSON
/// output — so structured CI consumers always receive a stable
/// envelope shape. A no-op for the human formatters (`pretty | github
/// | compact`), whose only failure signal is the stderr `error: …`
/// line (the dispatcher's `output::report`).
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
