//! Guest-side verb routing (RFC-61 Step 4, Milestone D).
//!
//! The workflow guest shim parses argv through the shared grammar and
//! hands the parsed [`crate::cli::Cli`] here. Pure workflow verbs run
//! in-process through the same handlers the native binary uses, so the
//! guest surface is argv- and envelope-compatible with native for
//! every shared verb. The four collapsed orchestrator verbs — the ones
//! whose native shape is a two-phase agent handoff — are *parsed* here
//! but *dispatched* in the shim, where the WIT-provided seam lives:
//! [`route`] returns the [`Orchestration`] descriptor and the shim
//! drives the matching `specify_workflow::orchestrate` entry point
//! against its providers. Native-only verbs (init, extension, lint,
//! workspace, `plan lock`, `slice build --phase`'s hook paths, …) have
//! no guest handler and fail with `Error::Argument` (exit 2).
//!
//! Guest-only argv semantics (documented divergence): `--phase` on
//! `source survey`, `source extract`, and `slice build` is accepted by
//! the shared grammar but ignored in-guest — the orchestrators collapse
//! prepare + finalize into one call, so there is no phase seam for an
//! agent to sit between.
//!
//! Project-scoped guest verbs anchor [`Ctx`] at `"."` — the mount
//! preopen that carries the project root — instead of walking from the
//! process CWD, which WASI does not model the way a native process
//! does.

use std::path::{Path, PathBuf};

use clap::Parser;
use specify_error::{Error, Result};

use crate::cli::{Cli, Commands, Format};
use crate::commands::journal::cli::JournalAction;
use crate::commands::plan::cli::PlanAction;
use crate::commands::slice::cli::{SliceAction, SliceMergeAction};
use crate::commands::source::cli::SourceAction;
use crate::commands::{self, journal};
use crate::context::Ctx;
use crate::output::{Exit, report};

/// How one parsed guest invocation runs.
#[derive(Debug)]
pub enum Route {
    /// The verb ran in-process (pure workflow verb, or a refusal);
    /// carry the exit through.
    Handled(Exit),
    /// A collapsed orchestrator verb: the shim drives it against its
    /// seam providers.
    Orchestrate(Orchestration),
}

/// One orchestrator dispatch the shim owns, with the global flags it
/// needs to load context and render output.
#[derive(Debug)]
pub struct Orchestration {
    /// Output format for the shim's envelope rendering.
    pub format: Format,
    /// Global `--plan-dir` plan-root override.
    pub plan_dir: Option<PathBuf>,
    /// The orchestrator verb to drive.
    pub verb: Verb,
}

/// The collapsed orchestrator verb surface. Argv-compatible with the
/// native two-phase verbs — same words, same flags — so the Step 5
/// skill thinning is a rename-free swap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verb {
    /// `specify source survey <source>` → `orchestrate::survey`.
    Survey {
        /// Source key from `plan.yaml.sources.<key>`.
        source: String,
        /// Plan name guard (`--plan`); when set, must match
        /// `plan.yaml.name`.
        plan: Option<String>,
    },
    /// `specify source extract <source> <lead> --slice <slice>` →
    /// `orchestrate::extract`.
    Extract {
        /// Source key from `plan.yaml.sources.<key>`.
        source: String,
        /// Lead id from `discovery.md`.
        lead: String,
        /// Slice the Evidence is extracted into.
        slice: String,
    },
    /// `specify slice build <name>` → `orchestrate::build`.
    Build {
        /// Slice name (under `.specify/slices/`).
        slice: String,
    },
    /// `specify slice merge run <name>` → `orchestrate::merge`
    /// (deterministic-only per RFC-61 decision D2; routed through the
    /// shim so every guest-vs-native behavioural divergence lives in
    /// one place).
    Merge {
        /// Slice name (under `.specify/slices/`).
        slice: String,
        /// Authorise a whole-document composition overwrite.
        allow_composition_replace: bool,
    },
}

/// Parse guest argv through the shared grammar.
///
/// This is the exact clap tree the native binary parses, so `--help`,
/// error text, and usage exits match verbatim. `argv` is passed as the
/// host provides it, `argv[0]` (the program name) included.
///
/// # Errors
///
/// On a parse failure (or `--help` / `--version`) clap's rendering is
/// written to the conventional sink (stdout for help/version, stderr
/// for usage errors) and the matching process exit code comes back as
/// an [`Exit`] for passthrough — the same codes the native binary
/// exits with.
pub fn parse(argv: impl IntoIterator<Item = String>) -> Result<Cli, Exit> {
    Cli::try_parse_from(argv).map_err(|err| {
        // clap's own printer keeps help on stdout and errors on
        // stderr; a sink failure leaves nothing better to do than
        // carry the exit code through.
        drop(err.print());
        Exit::Code(u8::try_from(err.exit_code()).unwrap_or(1))
    })
}

/// Route one parsed invocation for the guest: run pure verbs
/// in-process, describe orchestrator verbs for the shim, refuse
/// native-only verbs.
#[must_use]
pub fn route(cli: Cli) -> Route {
    let format = cli.format;
    let plan_dir = cli.plan_dir;
    let orchestrate = |verb: Verb| {
        Route::Orchestrate(Orchestration {
            format,
            plan_dir: plan_dir.clone(),
            verb,
        })
    };
    match cli.command {
        Commands::Source { action } => match action {
            // `--phase` is accepted but ignored in-guest (see the
            // module docs): the orchestrators collapse both phases.
            SourceAction::Survey { source, plan, .. } => orchestrate(Verb::Survey { source, plan }),
            SourceAction::Extract {
                source, lead, slice, ..
            } => orchestrate(Verb::Extract { source, lead, slice }),
            action => Route::Handled(commands::dispatch_source(format, plan_dir, action)),
        },
        Commands::Target { action } => Route::Handled(commands::dispatch_target(format, action)),
        // Inlined rather than `commands::dispatch_journal` on purpose:
        // the guest anchors `Ctx` at the `"."` preopen via the local
        // `scoped`, not `commands::scoped`'s CWD walk.
        Commands::Journal { action } => Route::Handled(match action {
            JournalAction::Emit { event, payload } => {
                scoped(format, plan_dir, |ctx| journal::emit::emit(ctx, &event, payload.as_deref()))
            }
            JournalAction::Show { filter, limit } => {
                scoped(format, plan_dir, |ctx| journal::show::show(ctx, filter.as_deref(), limit))
            }
        }),
        Commands::Plan { action } => match action {
            // No subprocesses in-guest: the lock fences separate OS
            // processes racing the plan, and the guest collapses every
            // breakout in-process (RFC-61 orchestrate posture).
            PlanAction::Lock { .. } => Route::Handled(unsupported(format, "plan lock")),
            action => {
                Route::Handled(scoped(format, plan_dir, |ctx| commands::plan::run(ctx, action)))
            }
        },
        Commands::Slice { action } => match action {
            SliceAction::Build { name, .. } => orchestrate(Verb::Build { slice: name }),
            SliceAction::Merge {
                action:
                    SliceMergeAction::Run {
                        name,
                        allow_composition_replace,
                    },
            } => orchestrate(Verb::Merge {
                slice: name,
                allow_composition_replace,
            }),
            action => {
                Route::Handled(scoped(format, plan_dir, |ctx| commands::slice::run(ctx, action)))
            }
        },
        Commands::Init { .. } => Route::Handled(unsupported(format, "init")),
        Commands::Adapter { .. } => Route::Handled(unsupported(format, "adapter")),
        Commands::Rules { .. } => Route::Handled(unsupported(format, "rules")),
        Commands::Extension { .. } => Route::Handled(unsupported(format, "extension")),
        Commands::Lint { .. } => Route::Handled(unsupported(format, "lint")),
        Commands::Catalog { .. } => Route::Handled(unsupported(format, "catalog")),
        Commands::Archive { .. } => Route::Handled(unsupported(format, "archive")),
        Commands::Registry { .. } => Route::Handled(unsupported(format, "registry")),
        Commands::Workspace { .. } => Route::Handled(unsupported(format, "workspace")),
        Commands::Completions { .. } => Route::Handled(unsupported(format, "completions")),
        Commands::Contract { .. } => Route::Handled(unsupported(format, "contract")),
        Commands::Upgrade { .. } => Route::Handled(unsupported(format, "upgrade")),
        Commands::Plugins { .. } => Route::Handled(unsupported(format, "plugins")),
    }
}

/// The guest counterpart of [`commands::scoped`]: load [`Ctx`]
/// anchored at `"."` (the project-root mount preopen) instead of
/// walking from the process CWD, keeping the same error-to-exit
/// boundary. Natively identical when the process runs from the
/// project root, which is exactly the native contract the guest
/// mirrors.
fn scoped<F>(format: Format, plan_dir: Option<PathBuf>, f: F) -> Exit
where
    F: FnOnce(&Ctx) -> Result<()>,
{
    let ctx = match Ctx::load_at(format, plan_dir, Path::new(".")) {
        Ok(ctx) => ctx,
        Err(err) => return report(format, &err),
    };
    match f(&ctx) {
        Ok(()) => Exit::Success,
        Err(err) => report(format, &err),
    }
}

/// Refuse a native-only verb on the standard argument-error surface
/// (wire code `argument`, exit 2) — no new wire code, and the guest's
/// stderr envelope matches the native binary's failure shape.
fn unsupported(format: Format, verb: &'static str) -> Exit {
    report(
        format,
        &Error::Argument {
            flag: "<command>",
            detail: format!(
                "`specify {verb}` is not available in the workflow guest; run it through the \
                 native binary"
            ),
        },
    )
}
