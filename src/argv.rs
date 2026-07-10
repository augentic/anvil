//! The guest shim's argv transport: the `wasi:cli/run` export plus
//! the exhaustive route match over [`Commands`].
//!
//! Structurally symmetric with `http.rs` — a transport entry (`struct
//! Cli` + `export!` + `Guest::run`) calling a route-table function.
//! One line per routed leaf: the parsed mirror `*Args` rides whole
//! into `cli::post` / `cli::get` (the [`cli::front::run`] bridge),
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

use cli::cli::{Commands, completions, parse};
use cli::commands::archive::cli::ArchiveAction;
use cli::commands::journal::cli::JournalAction;
use cli::commands::plan::cli::PlanAction;
use cli::commands::registry::cli::RegistryAction;
use cli::commands::slice::cli::{SliceAction, SliceMergeAction, SliceModelAction, SliceTaskAction};
use cli::commands::source::cli::SourceAction;
use cli::commands::target::cli::TargetAction;
use cli::output::{Exit, Format, report};
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
    ConflictCheck, Create, Drop, ModelShow, Overlap, Preview, Prune, Provenance, TaskMark,
    TaskProgress, Transition, TouchedSpecs, Validate,
};

use crate::provider::Provider;

/// The `wasi:cli/run` export struct (the clap parser root stays
/// qualified as `cli::cli::Cli`, never imported at this scope).
struct Cli;
wasip3::cli::command::export!(Cli);

impl wasip3::exports::cli::run::Guest for Cli {
    async fn run() -> Result<(), ()> {
        // argv verbatim as the host provides it, argv[0] included —
        // `parse` re-stamps argv[0] before clap sees it.
        let argv = wasip3::cli::environment::get_arguments();
        let cli = match parse(argv) {
            Ok(cli) => cli,
            Err(exit) => {
                // Exit-code passthrough on the p3 exit seam: `parse`
                // is `try_parse` under the hood so clap's usage-error
                // exit `2` survives (the p2 exit would collapse it).
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
fn preflight(cli: &cli::cli::Cli) -> Result<(), Error> {
    // Adapter metadata dispatch routes through this world's WIT
    // imports, so the resolvers work in-guest against the read-only
    // store and cache mounts.
    adapter::metadata::register(crate::provider::metadata);
    check_plan_dir(cli.plan_dir.as_deref())
}

/// Route one parsed invocation: convert the clap action to the
/// command's input DTO and run the handler; refuse the provisioning
/// commands.
async fn route(cli: cli::cli::Cli) -> Exit {
    let format = cli.format;
    if let Err(err) = preflight(&cli) {
        return report(format, &err);
    }
    let p = &Provider;
    match cli.command {
        Commands::Source { action } => match action {
            SourceAction::Resolve(args) => cli::get::<SourceResolve, _, _>(format, p, args).await,
            SourceAction::Survey(args) => cli::post::<Survey, _, _>(format, p, args).await,
            SourceAction::Extract(args) => cli::post::<Extract, _, _>(format, p, args).await,
        },
        Commands::Target { action } => match action {
            TargetAction::Resolve(args) => cli::get::<TargetResolve, _, _>(format, p, args).await,
        },
        Commands::Slice { action } => match action {
            SliceAction::Create(args) => cli::post::<Create, _, _>(format, p, args).await,
            SliceAction::Validate(args) => cli::get::<Validate, _, _>(format, p, args).await,
            SliceAction::Provenance(args) => cli::get::<Provenance, _, _>(format, p, args).await,
            SliceAction::Model { action } => match action {
                SliceModelAction::Show(args) => cli::get::<ModelShow, _, _>(format, p, args).await,
            },
            SliceAction::Refine(args) => cli::post::<Refine, _, _>(format, p, args).await,
            SliceAction::Build(args) => cli::post::<Build, _, _>(format, p, args).await,
            SliceAction::Merge { action } => match action {
                SliceMergeAction::Run(args) => cli::post::<MergeRun, _, _>(format, p, args).await,
                SliceMergeAction::Preview(args) => cli::get::<Preview, _, _>(format, p, args).await,
                SliceMergeAction::ConflictCheck(args) => {
                    cli::get::<ConflictCheck, _, _>(format, p, args).await
                }
            },
            SliceAction::Task { action } => match action {
                SliceTaskAction::Progress(args) => {
                    cli::get::<TaskProgress, _, _>(format, p, args).await
                }
                SliceTaskAction::Mark(args) => cli::post::<TaskMark, _, _>(format, p, args).await,
            },
            SliceAction::Transition(args) => cli::post::<Transition, _, _>(format, p, args).await,
            SliceAction::TouchedSpecs(args) => {
                cli::post::<TouchedSpecs, _, _>(format, p, args).await
            }
            SliceAction::Overlap(args) => cli::get::<Overlap, _, _>(format, p, args).await,
            SliceAction::Drop(args) => cli::post::<Drop, _, _>(format, p, args).await,
        },
        Commands::Plan { action } => match action {
            PlanAction::Create(args) => cli::post::<PlanCreate, _, _>(format, p, args).await,
            PlanAction::Validate(args) => cli::get::<PlanValidate, _, _>(format, p, args).await,
            PlanAction::Next(args) => cli::post::<Next, _, _>(format, p, args).await,
            PlanAction::Status(args) => cli::get::<Status, _, _>(format, p, args).await,
            PlanAction::Add(args) => cli::post::<PlanAdd, _, _>(format, p, args).await,
            PlanAction::Amend(args) => cli::post::<Amend, _, _>(format, p, args).await,
            PlanAction::Remove(args) => cli::post::<PlanRemove, _, _>(format, p, args).await,
            PlanAction::Transition(args) => cli::post::<PlanTransition, _, _>(format, p, args).await,
            PlanAction::Author(args) => cli::post::<Author, _, _>(format, p, args).await,
            PlanAction::Execute(args) => cli::post::<Execute, _, _>(format, p, args).await,
            PlanAction::Archive(args) => cli::post::<Archive, _, _>(format, p, args).await,
        },
        Commands::Journal { action } => match action {
            JournalAction::Emit(args) => cli::post::<Emit, _, _>(format, p, args).await,
            JournalAction::Show(args) => cli::get::<Show, _, _>(format, p, args).await,
        },
        Commands::Registry { action } => match action {
            RegistryAction::Validate(args) => {
                cli::get::<RegistryValidate, _, _>(format, p, args).await
            }
            RegistryAction::Add(args) => cli::post::<RegistryAdd, _, _>(format, p, args).await,
            RegistryAction::Remove(args) => cli::post::<RegistryRemove, _, _>(format, p, args).await,
        },
        Commands::Archive { action } => match action {
            ArchiveAction::Prune(args) => cli::post::<Prune, _, _>(format, p, args).await,
        },
        // The scaffold leg is the guest-invocable half of `init`:
        // project-scoped writes only, anchored at the `"."` mount
        // preopen. The provisioning half (`init` without the flag —
        // hydration, manifest generation) has no guest implementation
        // and refuses below.
        Commands::Init(args) if args.scaffold_only => {
            cli::post::<Scaffold, _, _>(format, p, args).await
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
