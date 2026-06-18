pub mod adapter;
pub mod agents;
pub mod archive;
pub mod catalog;
pub mod contract;
pub mod extension;
mod init;
pub mod journal;
pub mod lint;
pub mod plan;
pub mod plugins;
pub mod registry;
pub mod rules;
pub mod slice;
pub mod source;
pub mod target;
mod upgrade;
pub mod workspace;

use std::io::Write;
use std::path::{Path, PathBuf};

use clap::CommandFactory;
use serde::Serialize;
use specify_diagnostics::{
    Diagnostic, DiagnosticReport, DiagnosticReportVersion, DiagnosticSummary, blocking_present,
    renumber,
};
use specify_error::Result;
use specify_workflow::adapter::{Axis, SourceAdapter, TargetAdapter};
use specify_workflow::init::adapter_ref_from_value;

use crate::runtime::cli::{Cli, Commands, Format};
use crate::runtime::commands::contract::cli::ContractAction;
use crate::runtime::commands::extension::cli::ExtensionAction;
use crate::runtime::commands::journal::cli::JournalAction;
use crate::runtime::commands::lint::cli::LintAction;
use crate::runtime::commands::rules::cli::RulesAction;
use crate::runtime::commands::source::cli::SourceAction;
use crate::runtime::commands::target::cli::TargetAction;
use crate::runtime::commands::workspace::cli::WorkspaceAction;
use crate::runtime::context::Ctx;
use crate::runtime::output::{self, Exit, report};

pub fn run(cli: Cli) -> Exit {
    let format = cli.format;
    let plan_dir = cli.plan_dir;
    match cli.command {
        Commands::Init {
            adapter,
            name,
            description,
            workspace,
            include_framework,
            platforms,
            upgrade,
        } => dispatch(format, || {
            init::run(&init::Args {
                format,
                adapter: adapter.as_deref(),
                name: name.as_deref(),
                description: description.as_deref(),
                workspace,
                include_framework,
                platforms: platforms.as_deref(),
                upgrade,
            })
        }),
        Commands::Source { action } => dispatch_source(format, plan_dir, action),
        Commands::Target { action } => match action {
            TargetAction::Resolve { value, project_dir } => {
                dispatch(format, || resolve_adapter(format, Axis::Target, &value, &project_dir))
            }
        },
        Commands::Adapter { action } => dispatch_adapter(format, action),
        Commands::Rules { action } => match action {
            RulesAction::Export(args) => dispatch(format, || rules::export::run(format, &args)),
            RulesAction::Sync(args) => scoped(format, plan_dir, |ctx| rules::sync::run(ctx, &args)),
        },
        Commands::Extension { action } => match action {
            ExtensionAction::Run { name, args } => run_tool_with(format, &name, args),
            ExtensionAction::Fetch { name } => {
                scoped(format, plan_dir, |ctx| extension::fetch(ctx, name.as_deref()))
            }
            ExtensionAction::Gc => scoped(format, plan_dir, extension::gc),
            ExtensionAction::Schema { name, schema } => {
                run_tool_with(format, &name, vec!["schema".to_string(), schema])
            }
        },
        Commands::Lint { action } => dispatch_lint(format, action),
        Commands::Journal { action } => match action {
            JournalAction::Emit { event, payload } => {
                scoped(format, plan_dir, |ctx| journal::emit::emit(ctx, &event, payload.as_deref()))
            }
            JournalAction::Show { filter, limit } => {
                scoped(format, plan_dir, |ctx| journal::show::show(ctx, filter.as_deref(), limit))
            }
        },
        Commands::Slice { action } => scoped(format, plan_dir, |ctx| slice::run(ctx, action)),
        Commands::Catalog { action } => scoped(format, plan_dir, |ctx| catalog::run(ctx, action)),
        Commands::Archive { action } => scoped(format, plan_dir, |ctx| archive::run(ctx, &action)),
        Commands::Plan { action } => match action {
            // `plan lock` passes the wrapped child's exit code through
            // `Exit::Code`, so it bypasses the `Result<()>`-collapsing
            // `scoped` path the rest of the plan verbs share.
            plan::cli::PlanAction::Lock { command } => {
                run_plan_lock_with(format, plan_dir, &command)
            }
            action => scoped(format, plan_dir, |ctx| plan::run(ctx, action)),
        },
        Commands::Registry { action } => scoped(format, plan_dir, |ctx| registry::run(ctx, action)),
        Commands::Completions { shell } => {
            let mut cmd = Cli::command();
            clap_complete::generate(shell, &mut cmd, "specify", &mut std::io::stdout());
            Exit::Success
        }
        Commands::Contract { action } => match action {
            ContractAction::Dump => dispatch(format, || contract::dump::run(format)),
        },
        Commands::Upgrade {
            channel,
            yes,
            dry_run,
        } => dispatch(format, || upgrade::run(format, channel, yes, dry_run)),
        Commands::Plugins { action } => dispatch(format, || plugins::run(format, action)),
        Commands::Workspace { action } => match action {
            WorkspaceAction::Sync { projects } => {
                scoped(format, plan_dir, |ctx| workspace::sync(ctx, &projects))
            }
            WorkspaceAction::Prepare {
                project,
                change,
                sources,
                outputs,
            } => scoped(format, plan_dir, |ctx| {
                workspace::prepare(ctx, &project, change, sources, outputs)
            }),
            WorkspaceAction::Push { projects, dry_run } => {
                scoped(format, plan_dir, |ctx| workspace::push(ctx, &projects, dry_run))
            }
        },
    }
}

/// Dispatch the `specify adapter {build, publish}` family.
///
/// Factored out of [`run`] so the top-level dispatcher stays under the
/// per-function line budget (RFC-48 D6/D10).
fn dispatch_adapter(format: Format, action: adapter::cli::AdapterAction) -> Exit {
    match action {
        adapter::cli::AdapterAction::Build {
            path,
            dry_run,
            refresh_extension,
        } => dispatch(format, || adapter::build(format, &path, dry_run, refresh_extension)),
        adapter::cli::AdapterAction::Publish { path, reference } => {
            dispatch(format, || adapter::publish(format, &path, &reference))
        }
    }
}

/// Dispatch the `specify source {resolve, preview, survey, extract}`
/// family.
///
/// Factored out of [`run`] so the top-level dispatcher stays under the
/// per-function line budget; the arms keep their distinct context
/// posture — `resolve` / `preview` are project-context-free
/// ([`dispatch`]), `survey` / `extract` are project-scoped
/// ([`scoped`]).
fn dispatch_source(format: Format, plan_dir: Option<PathBuf>, action: SourceAction) -> Exit {
    match action {
        SourceAction::Resolve { name, project_dir } => {
            dispatch(format, || resolve_adapter(format, Axis::Source, &name, &project_dir))
        }
        SourceAction::Preview {
            adapter,
            source,
            lead,
            out,
            project_dir,
        } => dispatch(format, || {
            source::preview::preview(format, &adapter, &source, &lead, out.as_deref(), &project_dir)
        }),
        SourceAction::Survey { source, plan, phase } => scoped(format, plan_dir, |ctx| {
            source::survey::run(ctx, &source, plan.as_deref(), phase)
        }),
        SourceAction::Extract {
            source,
            lead,
            slice,
            phase,
        } => {
            scoped(format, plan_dir, |ctx| source::extract::run(ctx, &source, &lead, &slice, phase))
        }
    }
}

/// Dispatch the `specify lint {project, framework}` family.
fn dispatch_lint(format: Format, action: LintAction) -> Exit {
    match action {
        LintAction::Project(args) => {
            scoped_at(format, &args.project_dir, |ctx| lint::project::run(ctx, &args))
        }
        LintAction::Framework(args) => dispatch(format, || lint::framework::run(format, &args)),
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
fn scoped<F>(format: Format, plan_dir: Option<PathBuf>, f: F) -> Exit
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

/// Variant of [`scoped`] that loads `Ctx` against an explicit
/// project directory instead of the process CWD. Used by handlers
/// that take a `--project-dir` flag (e.g. `specify lint`), none of
/// which read plan artifacts — so no plan-root override is threaded.
fn scoped_at<F>(format: Format, project_dir: &Path, f: F) -> Exit
where
    F: FnOnce(&Ctx) -> Result<()>,
{
    let ctx = match Ctx::load_at(format, None, project_dir) {
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
fn dispatch<F>(format: Format, f: F) -> Exit
where
    F: FnOnce() -> Result<()>,
{
    match f() {
        Ok(()) => Exit::Success,
        Err(err) => report(format, &err),
    }
}

/// Extension execution is the only handler path that mints a [`Exit::Code`] exit;
/// see [DECISIONS.md §"Exit codes"](../DECISIONS.md#exit-codes) for
/// the rationale. Handled outside the `Result<()>` channel so the
/// success branch can carry the guest's exit code rather than
/// collapsing to `Success`.
fn run_tool_with(format: Format, name: &str, args: Vec<String>) -> Exit {
    let ctx = match Ctx::load(format, None) {
        Ok(ctx) => ctx,
        Err(err) => return report(format, &err),
    };
    match extension::run(&ctx, name, args) {
        Ok(0) => Exit::Success,
        Ok(code) => Exit::Code(code),
        Err(err) => report(format, &err),
    }
}

/// `specify plan lock -- <cmd>` runs a child under the plan lock and
/// passes its exit code through. Like [`run_tool_with`] it sits outside
/// the `Result<()>` channel so the success branch can carry the child's
/// own exit code rather than collapsing to `Success`.
fn run_plan_lock_with(format: Format, plan_dir: Option<PathBuf>, command: &[String]) -> Exit {
    let ctx = match Ctx::load(format, plan_dir) {
        Ok(ctx) => ctx,
        Err(err) => return report(format, &err),
    };
    match plan::lock::run(&ctx, command) {
        Ok(0) => Exit::Success,
        Ok(code) => Exit::Code(code),
        Err(err) => report(format, &err),
    }
}

/// Directory segment under a resolved adapter root that holds the
/// brief markdown files. Manifest brief paths are relative and join
/// onto `<adapter-root>/briefs/`. Shared with the source prep seam
/// ([`source::prep`]) so the C1 `briefs-dir` is computed in one place.
pub const BRIEFS_DIR: &str = "briefs";

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
    resolved_path: String,
    /// Absolute path to the resolved adapter's `briefs/` directory —
    /// `<resolved-path>/briefs`. Brief paths in the manifest are
    /// relative (e.g. `briefs/extract.md`) and join onto this root.
    briefs_dir: PathBuf,
    location: &'static str,
    operations: Vec<String>,
    description: Option<String>,
}

fn write_resolve_text(w: &mut dyn Write, body: &ResolveBody) -> std::io::Result<()> {
    writeln!(w, "{}", body.resolved_path)?;
    writeln!(w, "  axis: {}", body.axis)?;
    writeln!(w, "  name: {}", body.name)?;
    writeln!(w, "  briefs-dir: {}", body.briefs_dir.display())?;
    writeln!(w, "  location: {}", body.location)?;
    writeln!(w, "  operations: {}", body.operations.join(", "))?;
    if let Some(desc) = &body.description {
        writeln!(w, "  description: {desc}")?;
    }
    Ok(())
}

/// Resolve a source- or target-adapter manifest by kebab name and emit
/// the wire-stable [`ResolveBody`] envelope. Probe order matches the
/// axis-specific resolver: agent-populated out-of-tree manifest cache at
/// `<project-cache>/manifests/{sources,targets}/<name>/`
/// first, then the in-repo `<project_dir>/adapters/{sources,targets}/<name>/`.
///
/// For [`Axis::Target`], `value` accepts either `<name>` or
/// `<name>@<version>`; the `@version` suffix is treated as an opaque
/// identifier and stripped to leave the kebab name for the lookup
/// (workflow §CLI surface).
fn resolve_adapter(format: Format, axis: Axis, value: &str, project_dir: &Path) -> Result<()> {
    // Common envelope shape; only the per-axis resolver and the
    // `@version` strip (target-only) differ. `briefs_dir` is the
    // resolved adapter root joined with `briefs/` — the directory the
    // manifest's relative brief paths join onto (preview.rs:68).
    let (name, resolved_path, briefs_dir, location, operations, description) = match axis {
        Axis::Source => {
            let resolved = SourceAdapter::resolve(&adapter_ref_from_value(value), project_dir)?;
            let operations = resolved.manifest.operations().map(ToString::to_string).collect();
            let briefs_dir = resolved.location.path().join(BRIEFS_DIR);
            let resolved_path = resolved.location.path().display().to_string();
            let location = resolved.location.label();
            (
                resolved.manifest.name,
                resolved_path,
                briefs_dir,
                location,
                operations,
                resolved.manifest.description,
            )
        }
        Axis::Target => {
            let resolved = TargetAdapter::resolve(&adapter_ref_from_value(value), project_dir)?;
            let operations = resolved.manifest.operations().map(ToString::to_string).collect();
            let briefs_dir = resolved.location.path().join(BRIEFS_DIR);
            let resolved_path = resolved.location.path().display().to_string();
            let location = resolved.location.label();
            (
                resolved.manifest.name,
                resolved_path,
                briefs_dir,
                location,
                operations,
                resolved.manifest.description,
            )
        }
    };
    let body = ResolveBody {
        axis: axis.dir_segment(),
        name,
        resolved_path,
        briefs_dir,
        location,
        operations,
        description,
    };
    output::emit(&mut std::io::stdout().lock(), format, &body, write_resolve_text)?;
    Ok(())
}
