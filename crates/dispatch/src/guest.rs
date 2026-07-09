//! Guest-side verb routing — the whole operational surface.
//!
//! The workflow guest shim parses argv through the shared grammar and
//! hands the parsed [`crate::cli::Cli`] here. Pure workflow verbs run
//! in-process. The four collapsed orchestrator verbs — the ones whose
//! shape is a two-phase agent handoff — are *parsed* here but
//! *dispatched* in the shim, where the WIT-provided seam lives:
//! [`route`] returns the [`Orchestration`] descriptor and the shim
//! drives the matching `workflow::orchestrate` entry point
//! against its providers. Provisioning verbs (`init` without
//! `--scaffold-only`, `adapters sync`, `workspace *`, `upgrade`,
//! `plugins`) have no guest handler and fail with
//! `Error::Argument` (exit 2).
//!
//! Project-scoped guest verbs anchor [`Ctx`] at `"."` — the mount
//! preopen that carries the project root — instead of walking from the
//! process CWD, which WASI does not model the way a native process
//! does. The `--plan-dir` global is guarded here for the same reason:
//! plan artifacts anchor at the `"."` preopen, so any other plan root
//! would be silently ignored and is refused instead.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use clap::Parser;
use error::{Error, Result};
use workflow::change::SourceBinding;

use crate::cli::{Cli, Commands, Format, SourceArg};
use crate::commands::init::ScaffoldArgs;
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

/// The collapsed orchestrator verb surface — the same words and flags
/// the native binary parses before routing them to its guest leg.
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
    /// (deterministic-only; routed through the shim so every
    /// guest-vs-native behavioural divergence lives in one place).
    Merge {
        /// Slice name (under `.specify/slices/`).
        slice: String,
        /// Authorise a whole-document composition overwrite.
        allow_composition_replace: bool,
    },
    /// `specify plan execute` → `orchestrate::execute` (guest-only:
    /// the drained execute loop; the native binary refuses the verb).
    Execute,
    /// `specify plan author <name> [--source ...] [--intent ...]` →
    /// `orchestrate::author` (guest-only: the collapsed `/spec:plan`
    /// flow; the native binary refuses the verb).
    Author {
        /// Kebab-case change name.
        name: String,
        /// Desugared `--source` / `--intent` bindings — the same
        /// structured map `plan create` hands `Plan::init`.
        sources: BTreeMap<String, SourceBinding>,
    },
    /// `specify slice refine <name>` → `orchestrate::refine_breakout`
    /// (guest-only: the `/spec:refine` breakout outside the execute
    /// loop; the native binary refuses the verb).
    Refine {
        /// Slice name (a `plan.yaml.slices[]` entry).
        slice: String,
    },
}

/// Desugar the `plan author` argument surface into the structured
/// source-binding map — the `plan create` handler's rules verbatim:
/// `--intent` appends the value-bound intent binding before the
/// duplicate-key gate, so an explicit `--source intent=...` in the
/// same invocation trips `plan-source-duplicate-key`.
fn author_bindings(
    mut sources: Vec<SourceArg>, intent: Option<String>,
) -> Result<BTreeMap<String, SourceBinding>> {
    if let Some(value) = intent {
        sources.push(SourceArg::intent(value));
    }
    commands::plan::args::build_source_map(sources)
}

/// Parse guest argv through the shared grammar.
///
/// This is the exact clap tree the operator's `specify` invocation
/// forwards into, so `--help`, error text, and usage exits match a
/// native parse verbatim. The host supplies `argv[0]` as the
/// deployment's guest id, so it is replaced with the binary name
/// before parsing — clap renders `argv[0]` into every usage line, and
/// the operator typed `specify`, not `workflow`.
///
/// The parse is `try_parse` on purpose: `clap::Error::exit_code()`
/// travels back as an [`Exit`] the shim hands to the p3
/// `wasi:cli/exit`. `parse()`'s internal `process::exit` would land on
/// the p2 exit, which carries only success/failure and would collapse
/// clap's usage-error code `2` to `1`.
///
/// # Errors
///
/// On a parse failure (or `--help` / `--version`) clap's rendering is
/// written to the conventional sink (stdout for help/version, stderr
/// for usage errors) and the matching process exit code comes back as
/// an [`Exit`] for passthrough — the same codes the native binary
/// exits with.
pub fn parse(argv: impl IntoIterator<Item = String>) -> Result<Cli, Exit> {
    let forwarded = std::iter::once("specify".to_string()).chain(argv.into_iter().skip(1));
    Cli::try_parse_from(forwarded).map_err(|err| {
        // clap's own printer keeps help on stdout and errors on
        // stderr; a sink failure leaves nothing better to do than
        // carry the exit code through.
        drop(err.print());
        Exit::Code(u8::try_from(err.exit_code()).unwrap_or(1))
    })
}

/// Route one parsed invocation for the guest: run pure verbs
/// in-process, describe orchestrator verbs for the shim, refuse
/// provisioning verbs that have no guest implementation.
#[must_use]
pub fn route(cli: Cli) -> Route {
    let format = cli.format;
    let plan_dir = cli.plan_dir;
    if let Err(err) = check_plan_dir(plan_dir.as_deref()) {
        return Route::Handled(report(format, &err));
    }
    let orchestrate = |verb: Verb| {
        Route::Orchestrate(Orchestration {
            format,
            plan_dir: plan_dir.clone(),
            verb,
        })
    };
    match cli.command {
        Commands::Source { action } => match action {
            SourceAction::Survey { source, plan } => orchestrate(Verb::Survey { source, plan }),
            SourceAction::Extract { source, lead, slice } => {
                orchestrate(Verb::Extract { source, lead, slice })
            }
            action @ SourceAction::Resolve { .. } => {
                Route::Handled(commands::dispatch_source(format, plan_dir, action))
            }
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
            PlanAction::Execute => orchestrate(Verb::Execute),
            // Binding desugar failures (duplicate keys) surface on the
            // standard error envelope before any orchestration is
            // described.
            PlanAction::Author {
                name,
                sources,
                intent,
            } => match author_bindings(sources, intent) {
                Ok(bindings) => orchestrate(Verb::Author {
                    name,
                    sources: bindings,
                }),
                Err(err) => Route::Handled(report(format, &err)),
            },
            action => {
                Route::Handled(scoped(format, plan_dir, |ctx| commands::plan::run(ctx, action)))
            }
        },
        Commands::Slice { action } => match action {
            SliceAction::Build { name } => orchestrate(Verb::Build { slice: name }),
            SliceAction::Refine { name } => orchestrate(Verb::Refine { slice: name }),
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
        // The scaffold leg is the guest-invocable half of `init`:
        // project-scoped writes only, anchored at the `"."` mount
        // preopen. The provisioning half (`init` without the flag —
        // hydration, manifest generation) has no guest implementation
        // and refuses below.
        Commands::Init(args) if args.scaffold_only => Route::Handled(scaffold(
            format,
            args.adapter.as_deref(),
            args.name.as_deref(),
            args.description.as_deref(),
            args.workspace,
            args.platforms.as_deref(),
        )),
        Commands::Init(_) => Route::Handled(unsupported(format, "init")),
        Commands::Adapters { .. } => Route::Handled(unsupported(format, "adapters")),
        Commands::Archive { action } => {
            Route::Handled(scoped(format, plan_dir, |ctx| commands::archive::run(ctx, &action)))
        }
        Commands::Registry { action } => {
            Route::Handled(scoped(format, plan_dir, |ctx| commands::registry::run(ctx, action)))
        }
        Commands::Workspace { .. } => Route::Handled(unsupported(format, "workspace")),
        Commands::Completions { shell } => Route::Handled(commands::completions(shell)),
        Commands::Upgrade(_) => Route::Handled(unsupported(format, "upgrade")),
        Commands::Plugins { .. } => Route::Handled(unsupported(format, "plugins")),
    }
}

/// Refuse a `--plan-dir` (or `SPECIFY_PLAN_DIR`) pointing anywhere but
/// the project root: the guest anchors plan artifacts at the `"."`
/// mount preopen, so any other plan root would be silently ignored. A
/// value that resolves to the preopen itself is a no-op and passes.
fn check_plan_dir(plan_dir: Option<&Path>) -> Result<()> {
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

/// Run the init scaffold leg against the `"."` mount preopen — the
/// clap-to-[`ScaffoldArgs`] adaptor split out of [`route`] to keep the
/// router a flat table.
fn scaffold(
    format: Format, adapter: Option<&str>, name: Option<&str>, description: Option<&str>,
    workspace: bool, platforms: Option<&str>,
) -> Exit {
    commands::dispatch(format, || {
        commands::init::scaffold(
            Path::new("."),
            &ScaffoldArgs {
                format,
                adapter,
                name,
                description,
                workspace,
                platforms,
            },
        )
    })
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

/// Refuse a provisioning verb on the standard argument-error surface
/// (wire code `argument`, exit 2) — no new wire code. These verbs have
/// no in-guest implementation yet.
fn unsupported(format: Format, verb: &'static str) -> Exit {
    report(
        format,
        &Error::Argument {
            flag: "<command>",
            detail: format!("`specify {verb}` has no guest implementation yet"),
        },
    )
}
