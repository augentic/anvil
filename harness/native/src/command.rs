//! The native shim's argv transport: the process entry plus the
//! exhaustive route match over [`Commands`].
//!
//! Structurally symmetric with `http.rs` — a transport entry
//! ([`run`]) calling a route-table function. One line per routed
//! leaf: the parsed mirror `*Args` rides whole into `argv::post` /
//! `argv::get` (the [`argv::front::run`] bridge), which
//! serde-round-trips it onto the handler `Input` and drives the
//! handler against a [`NativeProvider`]. The refusal set is the
//! guest's exactly (parity-or-less): `init` without
//! `--scaffold-only`, `adapters`, `workspace`, `upgrade`, and
//! `plugins` refuse on the standard argument-error surface.
//!
//! The match is deliberately duplicated per shim (the wasm guest's
//! lives in the repo root's `src/command.rs`) so the compiler checks
//! each shim's coverage of the grammar — there is no shared route
//! table.

use std::fs;
use std::path::{Path, PathBuf};

use argv::cli::{Commands, completions, parse};
use argv::commands::archive::cli::ArchiveAction;
use argv::commands::journal::cli::JournalAction;
use argv::commands::plan::cli::PlanAction;
use argv::commands::registry::cli::RegistryAction;
use argv::commands::slice::cli::{SliceAction, SliceMergeAction, SliceModelAction, SliceTaskAction};
use argv::commands::source::cli::SourceAction;
use argv::commands::target::cli::TargetAction;
use argv::output::{Exit, Format, report};
use error::Error;
use specify_dev::mcp;
use specify_dev::model::DevModel;
use specify_dev::provider::NativeProvider;
use tokio::net::TcpListener;
use workflow::change::plan;
use workflow::{adapter, init, journal, orchestrate, registry, slice};

/// CLI-mode transport entry: parse argv through the shared grammar,
/// stand up the provider (with an ephemeral MCP shelf listener so
/// judgment legs carry real reference grants), and drive the matched
/// command to its numeric exit code.
pub async fn run(argv: Vec<String>) -> u8 {
    let cli = match parse(argv) {
        Ok(cli) => cli,
        Err(exit) => return exit.code(),
    };
    let format = cli.format;

    let root = PathBuf::from(".");
    let model = match DevModel::from_env(&root) {
        Ok(model) => model,
        Err(err) => {
            return report(
                format,
                &Error::Diag {
                    code: "dev-model-unavailable",
                    detail: format!("{err:#}"),
                },
            )
            .code();
        }
    };
    let mut provider = NativeProvider::new(root, model);
    if let Some(base) = shelves().await {
        provider = provider.mcp_base(base);
    }
    route(cli, &provider).await.code()
}

/// Serve the `/mcp/<name>` reference shelves on an ephemeral local
/// port for the lifetime of this invocation, returning the base URL.
/// A bind failure degrades to grant-less judgment, not an error.
async fn shelves() -> Option<String> {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.ok()?;
    let base = format!("http://127.0.0.1:{}", listener.local_addr().ok()?.port());
    tokio::spawn(async move {
        drop(axum::serve(listener, mcp::router()).await);
    });
    Some(base)
}

/// Route one parsed invocation: convert the clap action to the
/// command's input DTO and run the handler; refuse the provisioning
/// commands.
#[expect(clippy::too_many_lines, reason = "one arm per grammar leaf, exhaustive by design")]
async fn route(cli: argv::cli::Cli, p: &NativeProvider<DevModel>) -> Exit {
    let format = cli.format;
    if let Err(err) = check_plan_dir(cli.plan_dir.as_deref()) {
        return report(format, &err);
    }
    match cli.command {
        Commands::Source { action } => match action {
            SourceAction::Resolve(args) => {
                argv::get::<adapter::handlers::SourceResolve, _, _>(format, p, args).await
            }
            SourceAction::Survey(args) => {
                argv::post::<orchestrate::handlers::Survey, _, _>(format, p, args).await
            }
            SourceAction::Extract(args) => {
                argv::post::<orchestrate::handlers::Extract, _, _>(format, p, args).await
            }
        },
        Commands::Target { action } => match action {
            TargetAction::Resolve(args) => {
                argv::get::<adapter::handlers::TargetResolve, _, _>(format, p, args).await
            }
        },
        Commands::Slice { action } => match action {
            SliceAction::Create(args) => {
                argv::post::<slice::handlers::Create, _, _>(format, p, args).await
            }
            SliceAction::Validate(args) => {
                argv::get::<slice::handlers::Validate, _, _>(format, p, args).await
            }
            SliceAction::Provenance(args) => {
                argv::get::<slice::handlers::Provenance, _, _>(format, p, args).await
            }
            SliceAction::Model { action } => match action {
                SliceModelAction::Show(args) => {
                    argv::get::<slice::handlers::ModelShow, _, _>(format, p, args).await
                }
            },
            SliceAction::Refine(args) => {
                argv::post::<orchestrate::handlers::Refine, _, _>(format, p, args).await
            }
            SliceAction::Build(args) => {
                argv::post::<orchestrate::handlers::Build, _, _>(format, p, args).await
            }
            SliceAction::Merge { action } => match action {
                SliceMergeAction::Run(args) => {
                    argv::post::<orchestrate::handlers::MergeRun, _, _>(format, p, args).await
                }
                SliceMergeAction::Preview(args) => {
                    argv::get::<slice::handlers::Preview, _, _>(format, p, args).await
                }
                SliceMergeAction::ConflictCheck(args) => {
                    argv::get::<slice::handlers::ConflictCheck, _, _>(format, p, args).await
                }
            },
            SliceAction::Task { action } => match action {
                SliceTaskAction::Progress(args) => {
                    argv::get::<slice::handlers::TaskProgress, _, _>(format, p, args).await
                }
                SliceTaskAction::Mark(args) => {
                    argv::post::<slice::handlers::TaskMark, _, _>(format, p, args).await
                }
            },
            SliceAction::Transition(args) => {
                argv::post::<slice::handlers::Transition, _, _>(format, p, args).await
            }
            SliceAction::TouchedSpecs(args) => {
                argv::post::<slice::handlers::TouchedSpecs, _, _>(format, p, args).await
            }
            SliceAction::Overlap(args) => {
                argv::get::<slice::handlers::Overlap, _, _>(format, p, args).await
            }
            SliceAction::Drop(args) => {
                argv::post::<slice::handlers::Drop, _, _>(format, p, args).await
            }
        },
        Commands::Plan { action } => match action {
            PlanAction::Create(args) => {
                argv::post::<plan::handlers::Create, _, _>(format, p, args).await
            }
            PlanAction::Validate(args) => {
                argv::get::<plan::handlers::Validate, _, _>(format, p, args).await
            }
            PlanAction::Next(args) => {
                argv::post::<plan::handlers::Next, _, _>(format, p, args).await
            }
            PlanAction::Status(args) => {
                argv::get::<plan::handlers::Status, _, _>(format, p, args).await
            }
            PlanAction::Add(args) => argv::post::<plan::handlers::Add, _, _>(format, p, args).await,
            PlanAction::Amend(args) => {
                argv::post::<plan::handlers::Amend, _, _>(format, p, args).await
            }
            PlanAction::Remove(args) => {
                argv::post::<plan::handlers::Remove, _, _>(format, p, args).await
            }
            PlanAction::Transition(args) => {
                argv::post::<plan::handlers::Transition, _, _>(format, p, args).await
            }
            PlanAction::Author(args) => {
                argv::post::<orchestrate::handlers::Author, _, _>(format, p, args).await
            }
            PlanAction::Execute(args) => {
                argv::post::<orchestrate::handlers::Execute, _, _>(format, p, args).await
            }
            PlanAction::Archive(args) => {
                argv::post::<plan::handlers::Archive, _, _>(format, p, args).await
            }
        },
        Commands::Journal { action } => match action {
            JournalAction::Emit(args) => {
                argv::post::<journal::handlers::Emit, _, _>(format, p, args).await
            }
            JournalAction::Show(args) => {
                argv::get::<journal::handlers::Show, _, _>(format, p, args).await
            }
        },
        Commands::Registry { action } => match action {
            RegistryAction::Validate(args) => {
                argv::get::<registry::handlers::Validate, _, _>(format, p, args).await
            }
            RegistryAction::Add(args) => {
                argv::post::<registry::handlers::Add, _, _>(format, p, args).await
            }
            RegistryAction::Remove(args) => {
                argv::post::<registry::handlers::Remove, _, _>(format, p, args).await
            }
        },
        Commands::Archive { action } => match action {
            ArchiveAction::Prune(args) => {
                argv::post::<slice::handlers::Prune, _, _>(format, p, args).await
            }
        },
        // The scaffold leg only, matching the guest: the provisioning
        // half (hydration, manifest generation) stays with the shipped
        // path — a dev-shim-only implementation would fork the
        // operational surface off the wasm path.
        Commands::Init(args) if args.scaffold_only => {
            argv::post::<init::handlers::Scaffold, _, _>(format, p, args).await
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
/// the project root — the same policy as the guest, which anchors plan
/// artifacts at its `"."` preopen; the native shim anchors at the
/// working directory and keeps parity.
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
/// (wire code `argument`, exit 2) — the same set the guest refuses.
fn unsupported(format: Format, command: &'static str) -> Exit {
    report(
        format,
        &Error::Argument {
            flag: "<command>",
            detail: format!("`specify {command}` has no native dev-shim implementation"),
        },
    )
}
