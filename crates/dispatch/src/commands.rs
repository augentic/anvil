//! Shared verb handlers and dispatch plumbing.
//!
//! The guest router calls the per-family `dispatch_*` entry points
//! here for every pure workflow verb; provisioning verbs
//! (init's hydration half, adapters, workspace, upgrade, plugins)
//! keep only their clap action enums here (under each family's `cli`
//! module) so the grammar stays whole while the guest refuses them.
//! Guest-owned orchestrator verbs (`source survey`/`extract`,
//! `slice refine`/`build`, `slice merge run`, `plan author`/`execute`)
//! carry only their clap surface here — the workflow guest drives the
//! matching `workflow::orchestrate` entry points.

pub mod adapters;
pub mod archive;
pub mod init;
pub mod journal;
pub mod plan;
pub mod plugins;
pub mod registry;
pub mod rules;
pub mod slice;
pub mod source;
pub mod target;
pub mod workspace;

use std::io::Write;
use std::path::{Path, PathBuf};

use diagnostics::{
    Diagnostic, DiagnosticReport, DiagnosticReportVersion, DiagnosticSummary, blocking_present,
    renumber,
};
use error::Result;
use serde::Serialize;
use workflow::adapter::{Axis, SourceAdapter, TargetAdapter};
use workflow::init::adapter_ref_from_value;

use crate::cli::Format;
use crate::commands::journal::cli::JournalAction;
use crate::commands::source::cli::SourceAction;
use crate::commands::target::cli::TargetAction;
use crate::context::Ctx;
use crate::output::{self, Exit, report};

/// Dispatch the `specify source {resolve, survey, extract}` family.
///
/// Only `resolve` runs through the shared table (project-context-free,
/// [`dispatch`]); `survey` / `extract` are guest-owned collapsed
/// orchestrations peeled off by both dispatchers before this table —
/// the defensive arms keep the match exhaustive and never collapse a
/// real run to a misleading success.
pub fn dispatch_source(format: Format, _plan_dir: Option<PathBuf>, action: SourceAction) -> Exit {
    match action {
        SourceAction::Resolve { name, project_dir } => {
            dispatch(format, || resolve_adapter(format, Axis::Source, &name, &project_dir))
        }
        SourceAction::Survey { .. } => report(
            format,
            &error::Error::Argument {
                flag: "<command>",
                detail: "`specify source survey` dispatches outside the shared verb table"
                    .to_string(),
            },
        ),
        SourceAction::Extract { .. } => report(
            format,
            &error::Error::Argument {
                flag: "<command>",
                detail: "`specify source extract` dispatches outside the shared verb table"
                    .to_string(),
            },
        ),
    }
}

/// Dispatch the `specify target {resolve}` family.
pub fn dispatch_target(format: Format, action: TargetAction) -> Exit {
    match action {
        TargetAction::Resolve { value, project_dir } => {
            dispatch(format, || resolve_adapter(format, Axis::Target, &value, &project_dir))
        }
    }
}

/// Print the shell-completion script for `shell` to stdout — pure
/// stdout from the shared clap grammar, so the output is byte-identical
/// on both sides of the seam (native and in-guest, guest routing).
pub fn completions(shell: clap_complete::Shell) -> Exit {
    let mut cmd = <crate::cli::Cli as clap::CommandFactory>::command();
    clap_complete::generate(shell, &mut cmd, "specify", &mut std::io::stdout());
    Exit::Success
}

/// Dispatch the `specify journal {emit, show}` family.
pub fn dispatch_journal(format: Format, plan_dir: Option<PathBuf>, action: JournalAction) -> Exit {
    match action {
        JournalAction::Emit { event, payload } => {
            scoped(format, plan_dir, |ctx| journal::emit::emit(ctx, &event, payload.as_deref()))
        }
        JournalAction::Show { filter, limit } => {
            scoped(format, plan_dir, |ctx| journal::show::show(ctx, filter.as_deref(), limit))
        }
    }
}

/// Run a command that requires an initialised `.specify/` project.
///
/// Loads `Ctx` (project config + pipeline), calls `f`, and maps any
/// `Error` to the appropriate format-aware exit code via
/// [`report`]. This is the single error-handling boundary for
/// project-aware handlers — they can use `?` freely inside `f`.
/// `plan_dir` is the global `--plan-dir` plan-root override,
/// threaded into [`Ctx`] so `ctx.layout()` resolves plan artifacts
/// against it.
pub fn scoped<F>(format: Format, plan_dir: Option<PathBuf>, f: F) -> Exit
where
    F: FnOnce(&Ctx) -> Result<()>,
{
    let ctx = match Ctx::load(format, plan_dir) {
        Ok(ctx) => ctx,
        Err(err) => return report(format, &err),
    };
    match f(&ctx) {
        Ok(()) => Exit::Success,
        Err(err) => report(format, &err),
    }
}

/// Run a command that does NOT need project context but may still
/// fail with an `Error` (e.g. `source resolve` / `target resolve`).
/// The `Ctx`-bearing peer is [`scoped`].
pub fn dispatch<F>(format: Format, f: F) -> Exit
where
    F: FnOnce() -> Result<()>,
{
    match f() {
        Ok(()) => Exit::Success,
        Err(err) => report(format, &err),
    }
}

/// Render `findings` as a neutral [`DiagnosticReport`] on stdout in the
/// active `Ctx` format. JSON serialises the wire envelope
/// (`{ version, summary, findings }`); text renders a PASS/FAIL banner
/// plus one `row`-formatted line per finding. Ids are assigned
/// sequentially at render time. `empty_text`, when set, replaces the
/// banner entirely for a finding-free report (e.g. `Plan OK`). Shared
/// by `slice validate` and `plan validate`, which differ only in the
/// per-finding row formatter and the empty-report line.
fn render_diagnostic_report(
    ctx: &Ctx, mut findings: Vec<Diagnostic>, empty_text: Option<&'static str>,
    row: fn(&mut dyn Write, &Diagnostic) -> std::io::Result<()>,
) -> Result<()> {
    renumber(&mut findings);
    let blocking = blocking_present(&findings);
    let report = DiagnosticReport {
        version: DiagnosticReportVersion,
        summary: DiagnosticSummary::from_diagnostics(&findings),
        findings,
    };
    ctx.write(&report, move |w, report| {
        if report.findings.is_empty()
            && let Some(line) = empty_text
        {
            return writeln!(w, "{line}");
        }
        writeln!(w, "{}", if blocking { "FAIL" } else { "PASS" })?;
        for finding in &report.findings {
            row(w, finding)?;
        }
        Ok(())
    })
}

#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct ResolveBody {
    axis: &'static str,
    name: String,
    version: String,
    resolved_path: String,
    location: &'static str,
    operations: Vec<String>,
}

fn write_resolve_text(w: &mut dyn Write, body: &ResolveBody) -> std::io::Result<()> {
    writeln!(w, "{}", body.resolved_path)?;
    writeln!(w, "  axis: {}", body.axis)?;
    writeln!(w, "  name: {}", body.name)?;
    writeln!(w, "  version: {}", body.version)?;
    writeln!(w, "  location: {}", body.location)?;
    writeln!(w, "  operations: {}", body.operations.join(", "))?;
    Ok(())
}

/// Resolve a source- or target-adapter component by identity and emit
/// the wire-stable [`ResolveBody`] envelope. Probe order matches the
/// axis-agnostic locator: the global single-file store entry for
/// pinned identities, then the development release build for bare
/// names.
///
/// `value` accepts either `<name>` or `<name>@<version>`; the semver
/// `@version` suffix pins the identity to a store entry (workflow §CLI
/// surface).
fn resolve_adapter(format: Format, axis: Axis, value: &str, project_dir: &Path) -> Result<()> {
    // Common envelope shape; only the per-axis resolver differs.
    let (name, version, resolved_path, location, operations) = match axis {
        Axis::Source => {
            let resolved = SourceAdapter::resolve(&adapter_ref_from_value(value), project_dir)?;
            let operations = resolved.manifest.operations().map(ToString::to_string).collect();
            let resolved_path = resolved.location.path().display().to_string();
            let location = resolved.location.label();
            (
                resolved.manifest.name,
                resolved.manifest.version.to_string(),
                resolved_path,
                location,
                operations,
            )
        }
        Axis::Target => {
            let resolved = TargetAdapter::resolve(&adapter_ref_from_value(value), project_dir)?;
            let operations = resolved.manifest.operations().map(ToString::to_string).collect();
            let resolved_path = resolved.location.path().display().to_string();
            let location = resolved.location.label();
            (
                resolved.manifest.name,
                resolved.manifest.version.to_string(),
                resolved_path,
                location,
                operations,
            )
        }
    };
    let body = ResolveBody {
        axis: axis.dir_segment(),
        name,
        version,
        resolved_path,
        location,
        operations,
    };
    output::emit(&mut std::io::stdout().lock(), format, &body, write_resolve_text)?;
    Ok(())
}
