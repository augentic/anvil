//! Native `specify` dispatcher. Pure workflow verbs (`plan`, `slice`,
//! `source`, `target`, `journal`, `registry`, `archive`,
//! `rules export`, `completions`, init's scaffold leg) route to the
//! shared handlers in `specify-dispatch`; guest-owned orchestrator
//! verbs are peeled off by the triage layer (`commands::guest::owned`)
//! before this table; the native-only handlers below own everything
//! that needs subprocesses, Wasmtime, or the network (init's
//! provisioning half, adapters sync, lint, workspace, upgrade,
//! plugins).

mod adapters;
pub mod agents;
mod deploy;
pub mod describe;
pub mod guest;
mod init;
pub mod lint;
pub mod plugins;
pub mod rules;
mod upgrade;
pub mod workspace;

use specify_dispatch::commands::adapters::cli::AdaptersAction;
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
            scaffold_only,
        } => dispatch(format, || {
            // The hidden `--scaffold-only` form runs exactly the
            // shared scaffold leg the guest routes (RFC-65 move 1) —
            // no hydration, no manifest generation, no context
            // generation — so the two sides stay envelope-identical.
            if scaffold_only {
                return specify_dispatch::commands::init::scaffold(
                    std::path::Path::new("."),
                    &specify_dispatch::commands::init::ScaffoldArgs {
                        format,
                        adapter: adapter.as_deref(),
                        name: name.as_deref(),
                        description: description.as_deref(),
                        workspace,
                        include_framework,
                        platforms: platforms.as_deref(),
                    },
                );
            }
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
        Commands::Adapters { action } => match action {
            AdaptersAction::Sync { frozen } => {
                scoped(format, plan_dir, |ctx| adapters::sync(ctx, frozen))
            }
        },
        Commands::Source { action } => dispatch_source(format, plan_dir, action),
        Commands::Target { action } => dispatch_target(format, action),
        Commands::Rules { action } => match action {
            RulesAction::Export(args) => {
                dispatch(format, || specify_dispatch::commands::rules::export::run(format, &args))
            }
            RulesAction::Sync(args) => scoped(format, plan_dir, |ctx| rules::sync::run(ctx, args)),
        },
        Commands::Lint { action } => dispatch_lint(format, action),
        Commands::Journal { action } => dispatch_journal(format, plan_dir, action),
        Commands::Slice { action } => {
            scoped(format, plan_dir, |ctx| specify_dispatch::commands::slice::run(ctx, action))
        }
        Commands::Archive { action } => {
            scoped(format, plan_dir, |ctx| specify_dispatch::commands::archive::run(ctx, &action))
        }
        Commands::Plan { action } => {
            scoped(format, plan_dir, |ctx| specify_dispatch::commands::plan::run(ctx, action))
        }
        Commands::Registry { action } => {
            scoped(format, plan_dir, |ctx| specify_dispatch::commands::registry::run(ctx, action))
        }
        Commands::Completions { shell } => specify_dispatch::commands::completions(shell),
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

/// Dispatch the `specify lint {project, framework}` family.
fn dispatch_lint(format: Format, action: LintAction) -> Exit {
    match action {
        LintAction::Project(args) => {
            scoped_at(format, &args.project_dir, |ctx| lint::project::run(ctx, &args))
        }
        LintAction::Framework(args) => dispatch(format, || lint::framework::run(format, &args)),
    }
}
