//! Native `specify` dispatcher. Pure workflow verbs (`plan`, `slice`,
//! `source`, `target`, `journal`) route to the shared handlers in
//! `specify-dispatch`; guest-owned orchestrator verbs are peeled off
//! by the triage layer (`commands::guest::owned`) before this table;
//! the native-only handlers below own everything that needs
//! subprocesses, Wasmtime, or the network (init, lint, workspace, …).

pub mod adapter;
pub mod agents;
pub mod archive;
pub mod contract;
pub mod guest;
mod init;
pub mod lint;
pub mod plugins;
pub mod registry;
pub mod rules;
mod upgrade;
pub mod workspace;

use clap::CommandFactory;
use specify_dispatch::commands::adapter::cli::AdapterAction;
use specify_dispatch::commands::contract::cli::ContractAction;
use specify_dispatch::commands::lint::cli::LintAction;
use specify_dispatch::commands::rules::cli::RulesAction;
use specify_dispatch::commands::workspace::cli::WorkspaceAction;
use specify_dispatch::commands::{
    dispatch, dispatch_journal, dispatch_source, dispatch_target, scoped, scoped_at,
};

use crate::runtime::cli::{Cli, Commands, Format};
use crate::runtime::output::Exit;

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
        Commands::Target { action } => dispatch_target(format, action),
        Commands::Adapter { action } => dispatch_adapter(format, action),
        Commands::Rules { action } => match action {
            RulesAction::Export(args) => dispatch(format, || rules::export::run(format, &args)),
            RulesAction::Sync(args) => scoped(format, plan_dir, |ctx| rules::sync::run(ctx, &args)),
        },
        Commands::Lint { action } => dispatch_lint(format, action),
        Commands::Journal { action } => dispatch_journal(format, plan_dir, action),
        Commands::Slice { action } => {
            scoped(format, plan_dir, |ctx| specify_dispatch::commands::slice::run(ctx, action))
        }
        Commands::Archive { action } => scoped(format, plan_dir, |ctx| archive::run(ctx, &action)),
        Commands::Plan { action } => {
            scoped(format, plan_dir, |ctx| specify_dispatch::commands::plan::run(ctx, action))
        }
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
fn dispatch_adapter(format: Format, action: AdapterAction) -> Exit {
    match action {
        AdapterAction::Build { path } => dispatch(format, || adapter::build(format, &path)),
        AdapterAction::Publish { path, reference } => {
            dispatch(format, || adapter::publish(format, &path, &reference))
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
