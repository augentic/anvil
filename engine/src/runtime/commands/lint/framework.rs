//! `specify lint framework` handler — runs the declarative
//! deterministic-hint interpreter into a single [`DiagnosticReport`]
//! envelope.
//!
//! The shared pipeline lives in [`specify_standards::lint::runner`] and
//! the shared output/journal/exit tail in [`crate::output`]; this
//! handler is thin and obeys the same `Result<()>` contract as the
//! project `lint project` handler, differing only in the framework-surface
//! config it assembles:
//!
//! 1. Resolve and canonicalise the framework root (every `rules.*`
//!    check runs through the in-process `rules` checker, so no
//!    imperative producer is wired in).
//! 2. Configure the runner for the framework surface
//!    ([`ScanProfile::Framework`], [`NoopToolRunner`] — framework
//!    `kind: tool` rules resolve in-process, `include_core: true`).
//!    Codex resolution is fatal: a duplicate rule id or unresolvable
//!    rules tree aborts the run rather than silently passing.
//! 3. Hand the config to the shared [`crate::output::run_lint`] kernel
//!    (via [`crate::output::emit_lint_report`]), which renders the
//!    envelope, decides the blocking exit, and owns the JSON fallback on
//!    abort. This surface sets `journal: false`: framework self-lint is a
//!    development surface and the `lint-completed` journal contract is
//!    scoped to `specify lint project` (DECISIONS.md §"Journal event
//!    names").

use std::path::{Path, PathBuf};
use std::time::Instant;

use jiff::Timestamp;
use specify_diagnostics::{DiagnosticReport, Format as DiagnosticsFormat};
use specify_error::{Error, Result};
use specify_standards::ResolveInputs;
use specify_standards::lint::ScanProfile;
use specify_standards::lint::eval::NoopToolRunner;
use specify_standards::lint::runner::PipelineConfig;
use specify_workflow::config::Layout;
use specify_workflow::journal::LintScope;

use crate::output::{self, Format, LintRun};
use crate::runtime::commands::lint::cli::{FrameworkArgs, LintFormat};

/// Handler entry point dispatched from `src/runtime/commands.rs`.
///
/// Returns `Result<()>` like every runtime handler; the dispatcher maps
/// the terminal error through the shared `Exit::from(&Error)` table.
/// Always leaves a stable envelope on stdout for JSON output — the real
/// report on success, an empty all-zero envelope when the run aborts
/// before emit (via [`output::run_lint`]).
///
/// # Errors
///
/// Propagates the framework-root load error and any pipeline /
/// render abort routed through [`output::run_lint`].
pub fn run(format: Format, action: &FrameworkArgs) -> Result<()> {
    let diagnostics_format = pick_format(format, action.output_format);
    output::run_lint(diagnostics_format, || build_report(action, diagnostics_format))
}

/// Assemble the framework-surface inputs and config, then run the
/// shared pipeline + emit tail. Every `?` here is a pre-emit abort
/// that [`output::run_lint`] turns into the JSON fallback envelope.
fn build_report(
    action: &FrameworkArgs, format: DiagnosticsFormat,
) -> Result<Option<DiagnosticReport>> {
    let started_at = Instant::now();
    let project_dir = canonical_framework_root(&action.framework_root)?;

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

    // Every `rules.*` check (CORE-009 namespace ownership, CORE-026
    // duplicate id) runs through the in-process `rules` checker via
    // `kind: tool`, resolved in-process by the declarative pass — the
    // runner is never consulted for framework checkers.
    let rule_filter_slice: Vec<&str> = action.rules.iter().map(String::as_str).collect();
    let tool_runner = NoopToolRunner;
    let cli_contract = crate::runtime::commands::contract::dump::build_contract();
    let config = PipelineConfig {
        profile: ScanProfile::Framework,
        dump_model: action.dump_model,
        apply_ignore_directives: true,
        rule_filter: &rule_filter_slice,
        tool_runner: &tool_runner,
        cli_contract: Some(&cli_contract),
    };

    let scope = LintScope {
        target: None,
        slice: None,
        artifact: action.artifacts.first().map(|p| p.display().to_string()),
    };
    output::emit_lint_report(LintRun {
        inputs: &inputs,
        config: &config,
        format,
        layout: Layout::new(&project_dir),
        now: Timestamp::now(),
        scope,
        // Framework self-lint is a development surface; the `lint-completed`
        // journal contract is scoped to `specify lint project` (DECISIONS.md
        // §"Journal event names"), so this surface never journals.
        journal: false,
        command_label: "specify lint framework",
        started_at,
        trailing_newline: false,
    })
}

/// Canonicalise the framework root after a structural sanity check
/// (the `adapters/` directory, with or without `plugins/`), so every
/// downstream path in the report is anchored at a stable absolute root.
/// An adapters-only root is a first-class framework root: the
/// plugin-bound checkers no-op when their inputs are absent.
fn canonical_framework_root(root: &Path) -> Result<PathBuf> {
    if !root.join("adapters").is_dir() {
        return Err(Error::Diag {
            code: "framework-root",
            detail: format!("not a framework root: {}", root.display()),
        });
    }
    root.canonicalize().map_err(|source| Error::Diag {
        code: "framework-root",
        detail: format!("canonicalize path: {source}"),
    })
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
