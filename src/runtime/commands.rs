//! Native `specify` dispatcher — the provisioning surface only. The
//! table below serves the closed native set (`init`, `adapters sync`,
//! `upgrade`, `plugins`), the acknowledged `workspace` residue, and
//! the hidden `lint framework` dev tool. Every workflow verb reaches
//! this binary as unparsed argv and forwards to the workflow guest
//! (`commands::guest::forward`) before this table is ever consulted.

mod adapters;
pub mod agents;
mod deploy;
pub mod describe;
pub mod guest;
mod init;
pub mod lint;
pub mod plugins;
mod upgrade;
pub mod workspace;

use specify_dispatch::commands::adapters::cli::AdaptersAction;
use specify_dispatch::commands::lint::cli::LintAction;
use specify_dispatch::commands::workspace::cli::WorkspaceAction;
use specify_dispatch::commands::{dispatch, scoped};

use crate::runtime::cli::{Cli, Commands};
use crate::runtime::output::Exit;

pub fn run(cli: Cli) -> Exit {
    let format = cli.format;
    let plan_dir = cli.plan_dir;
    match cli.command {
        Commands::Init(args) => dispatch(format, || {
            // The hidden `--scaffold-only` form runs exactly the
            // shared scaffold leg the guest routes (RFC-65 move 1) —
            // no hydration, no manifest generation, no context
            // generation — so the two sides stay envelope-identical.
            if args.scaffold_only {
                return specify_dispatch::commands::init::scaffold(
                    std::path::Path::new("."),
                    &specify_dispatch::commands::init::ScaffoldArgs {
                        format,
                        adapter: args.adapter.as_deref(),
                        name: args.name.as_deref(),
                        description: args.description.as_deref(),
                        workspace: args.workspace,
                        include_framework: args.include_framework,
                        platforms: args.platforms.as_deref(),
                    },
                );
            }
            init::run(&init::Args {
                format,
                adapter: args.adapter.as_deref(),
                name: args.name.as_deref(),
                description: args.description.as_deref(),
                workspace: args.workspace,
                include_framework: args.include_framework,
                platforms: args.platforms.as_deref(),
                upgrade: args.upgrade,
            })
        }),
        Commands::Adapters { action } => match action {
            AdaptersAction::Sync { frozen } => {
                scoped(format, plan_dir, |ctx| adapters::sync(ctx, frozen))
            }
        },
        Commands::Upgrade(args) => {
            dispatch(format, || upgrade::run(format, args.channel, args.yes, args.dry_run))
        }
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
        Commands::Lint { action } => match action {
            LintAction::Framework(args) => dispatch(format, || lint::framework::run(format, &args)),
        },
    }
}
