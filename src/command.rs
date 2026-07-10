//! The guest shim's argv transport: the `wasi:cli/run` export plus
//! the exhaustive route match over [`Commands`].
//!
//! Structurally symmetric with `http.rs` — a transport entry (`struct
//! Cli` + `export!` + `Guest::run`) calling a route-table function.
//! One line per routed leaf: the parsed mirror `*Args` rides whole
//! into `argv::run` / `argv::run` (the [`argv::front::run`] bridge),
//! which serde-round-trips it onto the handler `Input` and drives the
//! handler against [`Provider`] — the WIT-backed `Anchor + Model +
//! SourceSeam + TargetSeam` implementation. Provisioning commands
//! (`init` without `--scaffold-only`, `adapters`, `workspace`,
//! `upgrade`, `plugins`) have no guest implementation and refuse with
//! `Error::Argument` (exit 2).
//!
//! The match is deliberately duplicated per shim (the native
//! `specify-dev` binary carries its own) so the compiler checks each
//! shim's coverage of the grammar — there is no shared route table.

use std::fs;
use std::path::Path;

use argv::cli::{Cli, Commands, completions, parse};
use argv::commands::archive::cli::ArchiveAction;
use argv::commands::journal::cli::JournalAction;
use argv::commands::plan::cli::PlanAction;
use argv::commands::registry::cli::RegistryAction;
use argv::commands::slice::cli::{SliceAction, SliceMergeAction, SliceModelAction, SliceTaskAction};
use argv::commands::source::cli::SourceAction;
use argv::commands::target::cli::TargetAction;
use argv::output::{Exit, Format, report};
use error::Error;
use omnia_guest::wasip3;
use workflow::adapter;
use workflow::adapter::handlers::{SourceResolve, TargetResolve};
use workflow::change::plan::handlers::{
    Add as PlanAdd, Amend, Archive, Create as PlanCreate, Next, Remove as PlanRemove, Status,
    Transition as PlanTransition, Validate as PlanValidate,
};
use workflow::init::handlers::Scaffold;
use workflow::journal::handlers::{Emit, Show};
use workflow::orchestrate::handlers::{Author, Build, Execute, Extract, MergeRun, Refine, Survey};
use workflow::registry::handlers::{
    Add as RegistryAdd, Remove as RegistryRemove, Validate as RegistryValidate,
};
use workflow::slice::handlers::{
    ConflictCheck, Create, Drop, ModelShow, Overlap, Preview, Provenance, Prune, TaskMark,
    TaskProgress, TouchedSpecs, Transition, Validate,
};

use crate::provider::Provider;

struct CliGuest;
wasip3::cli::command::export!(CliGuest);

impl wasip3::exports::cli::run::Guest for CliGuest {
    async fn run() -> Result<(), ()> {
        let argv = wasip3::cli::environment::get_arguments();
        let cli = match parse(argv) {
            Ok(cli) => cli,
            Err(exit) => {
                wasip3::cli::exit::exit_with_code(exit.code());
                unreachable!("exit_with_code does not return");
            }
        };
        let exit = route(cli).await;
        if exit.code() == 0 {
            Ok(())
        } else {
            wasip3::cli::exit::exit_with_code(exit.code());
            unreachable!("exit_with_code does not return");
        }
    }
}

/// Shim-edge policy shared by every arm: register the in-guest
/// metadata runner (idempotent) and refuse a foreign `--plan-dir`.
fn preflight(cli: &Cli) -> Result<(), Error> {
    // Adapter metadata dispatch routes through this world's WIT
    // imports, so the resolvers work in-guest against the read-only
    // store and cache mounts.
    adapter::metadata::register(crate::provider::metadata);
    check_plan_dir(cli.plan_dir.as_deref())
}

/// Route one parsed invocation: convert the clap action to the
/// command's input DTO and run the handler; refuse the provisioning
/// commands.
async fn route(cli: Cli) -> Exit {
    let format = cli.format;
    if let Err(err) = preflight(&cli) {
        return report(format, &err);
    }

    let p = &Provider;
    match cli.command {
        Commands::Source { action } => match action {
            SourceAction::Resolve(args) => argv::run::<SourceResolve, _, _>(format, p, args).await,
            SourceAction::Survey(args) => argv::run::<Survey, _, _>(format, p, args).await,
            SourceAction::Extract(args) => argv::run::<Extract, _, _>(format, p, args).await,
        },
        Commands::Target { action } => match action {
            TargetAction::Resolve(args) => argv::run::<TargetResolve, _, _>(format, p, args).await,
        },
        Commands::Slice { action } => match action {
            SliceAction::Create(args) => argv::run::<Create, _, _>(format, p, args).await,
            SliceAction::Validate(args) => argv::run::<Validate, _, _>(format, p, args).await,
            SliceAction::Provenance(args) => argv::run::<Provenance, _, _>(format, p, args).await,
            SliceAction::Model { action } => match action {
                SliceModelAction::Show(args) => argv::run::<ModelShow, _, _>(format, p, args).await,
            },
            SliceAction::Refine(args) => argv::run::<Refine, _, _>(format, p, args).await,
            SliceAction::Build(args) => argv::run::<Build, _, _>(format, p, args).await,
            SliceAction::Merge { action } => match action {
                SliceMergeAction::Run(args) => argv::run::<MergeRun, _, _>(format, p, args).await,
                SliceMergeAction::Preview(args) => argv::run::<Preview, _, _>(format, p, args).await,
                SliceMergeAction::ConflictCheck(args) => {
                    argv::run::<ConflictCheck, _, _>(format, p, args).await
                }
            },
            SliceAction::Task { action } => match action {
                SliceTaskAction::Progress(args) => {
                    argv::run::<TaskProgress, _, _>(format, p, args).await
                }
                SliceTaskAction::Mark(args) => argv::run::<TaskMark, _, _>(format, p, args).await,
            },
            SliceAction::Transition(args) => argv::run::<Transition, _, _>(format, p, args).await,
            SliceAction::TouchedSpecs(args) => {
                argv::run::<TouchedSpecs, _, _>(format, p, args).await
            }
            SliceAction::Overlap(args) => argv::run::<Overlap, _, _>(format, p, args).await,
            SliceAction::Drop(args) => argv::run::<Drop, _, _>(format, p, args).await,
        },
        Commands::Plan { action } => match action {
            PlanAction::Create(args) => argv::run::<PlanCreate, _, _>(format, p, args).await,
            PlanAction::Validate(args) => argv::run::<PlanValidate, _, _>(format, p, args).await,
            PlanAction::Next(args) => argv::run::<Next, _, _>(format, p, args).await,
            PlanAction::Status(args) => argv::run::<Status, _, _>(format, p, args).await,
            PlanAction::Add(args) => argv::run::<PlanAdd, _, _>(format, p, args).await,
            PlanAction::Amend(args) => argv::run::<Amend, _, _>(format, p, args).await,
            PlanAction::Remove(args) => argv::run::<PlanRemove, _, _>(format, p, args).await,
            PlanAction::Transition(args) => {
                argv::run::<PlanTransition, _, _>(format, p, args).await
            }
            PlanAction::Author(args) => argv::run::<Author, _, _>(format, p, args).await,
            PlanAction::Execute(args) => argv::run::<Execute, _, _>(format, p, args).await,
            PlanAction::Archive(args) => argv::run::<Archive, _, _>(format, p, args).await,
        },
        Commands::Journal { action } => match action {
            JournalAction::Emit(args) => argv::run::<Emit, _, _>(format, p, args).await,
            JournalAction::Show(args) => argv::run::<Show, _, _>(format, p, args).await,
        },
        Commands::Registry { action } => match action {
            RegistryAction::Validate(args) => {
                argv::run::<RegistryValidate, _, _>(format, p, args).await
            }
            RegistryAction::Add(args) => argv::run::<RegistryAdd, _, _>(format, p, args).await,
            RegistryAction::Remove(args) => {
                argv::run::<RegistryRemove, _, _>(format, p, args).await
            }
        },
        Commands::Archive { action } => match action {
            ArchiveAction::Prune(args) => argv::run::<Prune, _, _>(format, p, args).await,
        },
        // The scaffold leg is the guest-invocable half of `init`:
        // project-scoped writes only, anchored at the `"."` mount
        // preopen. The provisioning half (`init` without the flag —
        // hydration, manifest generation) has no guest implementation
        // and refuses below.
        Commands::Init(args) if args.scaffold_only => {
            argv::run::<Scaffold, _, _>(format, p, args).await
        }
        Commands::Init(_) => unsupported(format, "init"),
        Commands::Adapters { .. } => unsupported(format, "adapters"),
        Commands::Workspace { .. } => unsupported(format, "workspace"),
        Commands::Upgrade(_) => unsupported(format, "upgrade"),
        Commands::Plugins { .. } => unsupported(format, "plugins"),
        Commands::Completions { shell } => completions(shell),
    }
}

/// Refuse a `--plan-dir` (or `SPECIFY_PLAN_DIR`) pointing anywhere but
/// the project root: the guest anchors plan artifacts at the `"."`
/// mount preopen, so any other plan root would be silently ignored. A
/// value that resolves to the preopen itself is a no-op and passes.
fn check_plan_dir(plan_dir: Option<&Path>) -> Result<(), Error> {
    let Some(dir) = plan_dir else {
        return Ok(());
    };
    let same = dir == Path::new(".")
        || fs::canonicalize(dir)
            .and_then(|requested| fs::canonicalize(".").map(|root| requested == root))
            .unwrap_or(false);
    if same {
        return Ok(());
    }
    Err(Error::Argument {
        flag: "--plan-dir",
        detail: format!(
            "`--plan-dir` must be the project root: plan artifacts anchor at the working \
             directory, so {} would be ignored; run from the plan root instead",
            dir.display()
        ),
    })
}

/// Refuse a provisioning command on the standard argument-error surface
/// (wire code `argument`, exit 2) — no new wire code. These commands have
/// no in-guest implementation yet.
fn unsupported(format: Format, command: &'static str) -> Exit {
    report(
        format,
        &Error::Argument {
            flag: "<command>",
            detail: format!("`specify {command}` has no guest implementation yet"),
        },
    )
}
