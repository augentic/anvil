//! The native shim's exhaustive dispatch match over [`Commands`].
//!
//! One arm per leaf command, converting the parsed clap action into the
//! matching handler `Input` DTO and driving the handler through
//! [`cli::front::run`] against a [`NativeProvider`]. The refusal set is
//! the guest's exactly (parity-or-less): `init` without
//! `--scaffold-only`, `adapters`, `workspace`, `upgrade`, and
//! `plugins` refuse on the standard argument-error surface.
//!
//! The match is deliberately duplicated per shim (the wasm guest's
//! lives in the repo root's `src/dispatch.rs`) so the compiler checks
//! each shim's coverage of the grammar — there is no shared route
//! table.

use std::fs;
use std::path::{Path, PathBuf};

use cli::cli::{Cli, Commands, completions, parse};
use cli::commands::archive::cli::ArchiveAction;
use cli::commands::journal::cli::JournalAction;
use cli::commands::plan::cli::PlanAction;
use cli::commands::plan::{assigns, bindings, source_map};
use cli::commands::registry::cli::RegistryAction;
use cli::commands::slice::cli::{SliceAction, SliceMergeAction, SliceModelAction, SliceTaskAction};
use cli::commands::source::cli::SourceAction;
use cli::commands::target::cli::TargetAction;
use cli::front::run;
use cli::output::{Exit, Format, report};
use error::Error;
use specify_dev::mcp;
use specify_dev::model::DevModel;
use specify_dev::provider::NativeProvider;
use tokio::net::TcpListener;
use workflow::change::plan;
use workflow::{adapter, init, journal, orchestrate, registry, slice};

/// CLI-mode entry: parse argv through the shared grammar, stand up the
/// provider (with an ephemeral MCP shelf listener so judgment legs
/// carry real reference grants), and drive the matched command to its
/// numeric exit code.
pub async fn main(argv: Vec<String>) -> u8 {
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
    dispatch(cli, &provider).await.code()
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

/// Route one parsed invocation: convert the clap action to the command's/// input DTO and run the handler; refuse the provisioning commands.
#[expect(clippy::too_many_lines, reason = "one arm per grammar leaf, exhaustive by design")]
async fn dispatch(cli: Cli, p: &NativeProvider<DevModel>) -> Exit {
    let format = cli.format;
    if let Err(err) = check_plan_dir(cli.plan_dir.as_deref()) {
        return report(format, &err);
    }
    match cli.command {
        Commands::Source { action } => match action {
            SourceAction::Resolve { name, project_dir } => {
                run::<adapter::handlers::SourceResolve, _, _>(
                    format,
                    p,
                    adapter::handlers::ResolveInput {
                        value: name,
                        project_dir: Some(project_dir),
                    },
                )
                .await
            }
            SourceAction::Survey { source, plan } => {
                run::<orchestrate::handlers::Survey, _, _>(
                    format,
                    p,
                    orchestrate::handlers::SurveyInput { source, plan },
                )
                .await
            }
            SourceAction::Extract { source, lead, slice } => {
                run::<orchestrate::handlers::Extract, _, _>(
                    format,
                    p,
                    orchestrate::handlers::ExtractInput { source, lead, slice },
                )
                .await
            }
        },
        Commands::Target { action } => match action {
            TargetAction::Resolve { value, project_dir } => {
                run::<adapter::handlers::TargetResolve, _, _>(
                    format,
                    p,
                    adapter::handlers::ResolveInput {
                        value,
                        project_dir: Some(project_dir),
                    },
                )
                .await
            }
        },
        Commands::Slice { action } => match action {
            SliceAction::Create {
                name,
                target,
                if_exists,
            } => {
                run::<slice::handlers::Create, _, _>(
                    format,
                    p,
                    slice::handlers::CreateInput {
                        name,
                        target,
                        if_exists: if_exists.to_string(),
                    },
                )
                .await
            }
            SliceAction::Validate { name } => {
                run::<slice::handlers::Validate, _, _>(
                    format,
                    p,
                    slice::handlers::ValidateInput { name },
                )
                .await
            }
            SliceAction::Provenance { name } => {
                run::<slice::handlers::Provenance, _, _>(
                    format,
                    p,
                    slice::handlers::ProvenanceInput { name },
                )
                .await
            }
            SliceAction::Model { action } => match action {
                SliceModelAction::Show { name } => {
                    run::<slice::handlers::ModelShow, _, _>(
                        format,
                        p,
                        slice::handlers::ModelShowInput { name },
                    )
                    .await
                }
            },
            SliceAction::Refine { name } => {
                run::<orchestrate::handlers::Refine, _, _>(
                    format,
                    p,
                    orchestrate::handlers::RefineInput { name },
                )
                .await
            }
            SliceAction::Build { name } => {
                run::<orchestrate::handlers::Build, _, _>(
                    format,
                    p,
                    orchestrate::handlers::BuildInput { name },
                )
                .await
            }
            SliceAction::Merge { action } => match action {
                SliceMergeAction::Run {
                    name,
                    allow_composition_replace,
                } => {
                    run::<orchestrate::handlers::MergeRun, _, _>(
                        format,
                        p,
                        orchestrate::handlers::MergeRunInput {
                            name,
                            allow_composition_replace,
                        },
                    )
                    .await
                }
                SliceMergeAction::Preview { name } => {
                    run::<slice::handlers::Preview, _, _>(
                        format,
                        p,
                        slice::handlers::PreviewInput { name },
                    )
                    .await
                }
                SliceMergeAction::ConflictCheck { name } => {
                    run::<slice::handlers::ConflictCheck, _, _>(
                        format,
                        p,
                        slice::handlers::ConflictCheckInput { name },
                    )
                    .await
                }
            },
            SliceAction::Task { action } => match action {
                SliceTaskAction::Progress { name } => {
                    run::<slice::handlers::TaskProgress, _, _>(
                        format,
                        p,
                        slice::handlers::TaskProgressInput { name },
                    )
                    .await
                }
                SliceTaskAction::Mark { name, task_number } => {
                    run::<slice::handlers::TaskMark, _, _>(
                        format,
                        p,
                        slice::handlers::TaskMarkInput { name, task_number },
                    )
                    .await
                }
            },
            SliceAction::Transition { name, target } => {
                run::<slice::handlers::Transition, _, _>(
                    format,
                    p,
                    slice::handlers::TransitionInput { name, target },
                )
                .await
            }
            SliceAction::TouchedSpecs { name, scan, set } => {
                run::<slice::handlers::TouchedSpecs, _, _>(
                    format,
                    p,
                    slice::handlers::TouchedSpecsInput { name, scan, set },
                )
                .await
            }
            SliceAction::Overlap { name } => {
                run::<slice::handlers::Overlap, _, _>(
                    format,
                    p,
                    slice::handlers::OverlapInput { name },
                )
                .await
            }
            SliceAction::Drop { name, reason } => {
                run::<slice::handlers::Drop, _, _>(
                    format,
                    p,
                    slice::handlers::DropInput { name, reason },
                )
                .await
            }
        },
        Commands::Plan { action } => match action {
            PlanAction::Create {
                name,
                sources,
                intent,
                auto_approve,
                authority_override,
            } => match source_map(sources, intent) {
                Ok(sources) => {
                    run::<plan::handlers::Create, _, _>(
                        format,
                        p,
                        plan::handlers::CreateInput {
                            name,
                            sources,
                            auto_approve,
                            authority_override,
                        },
                    )
                    .await
                }
                Err(err) => report(format, &err),
            },
            PlanAction::Validate => {
                run::<plan::handlers::Validate, _, _>(format, p, plan::handlers::ValidateInput {})
                    .await
            }
            PlanAction::Next => {
                run::<plan::handlers::Next, _, _>(format, p, plan::handlers::NextInput {}).await
            }
            PlanAction::Status => {
                run::<plan::handlers::Status, _, _>(format, p, plan::handlers::StatusInput {}).await
            }
            PlanAction::Add(args) => {
                run::<plan::handlers::Add, _, _>(
                    format,
                    p,
                    plan::handlers::AddInput {
                        name: args.name,
                        depends_on: args.depends_on,
                        sources: bindings(args.sources),
                        description: args.description,
                        project: args.project,
                        context: args.context,
                        authority_override: assigns(args.authority_override),
                    },
                )
                .await
            }
            PlanAction::Amend(args) => {
                run::<plan::handlers::Amend, _, _>(
                    format,
                    p,
                    plan::handlers::AmendInput {
                        name: args.name,
                        depends_on: args.depends_on,
                        sources: args.sources.map(bindings),
                        add_source: bindings(args.add_source),
                        remove_source: args.remove_source,
                        divergence: args.divergence,
                        description: args.description,
                        project: args.project,
                        context: args.context,
                        authority_override: args.authority_override,
                        clear_authority_override: args.clear_authority_override,
                        clear_authority_overrides: args.clear_authority_overrides,
                    },
                )
                .await
            }
            PlanAction::Remove { name } => {
                run::<plan::handlers::Remove, _, _>(format, p, plan::handlers::RemoveInput { name })
                    .await
            }
            PlanAction::Transition {
                name,
                target,
                undo,
                actor,
            } => {
                run::<plan::handlers::Transition, _, _>(
                    format,
                    p,
                    plan::handlers::TransitionInput {
                        name,
                        target,
                        undo,
                        actor,
                    },
                )
                .await
            }
            PlanAction::Author {
                name,
                sources,
                intent,
            } => match source_map(sources, intent) {
                Ok(sources) => {
                    run::<orchestrate::handlers::Author, _, _>(
                        format,
                        p,
                        orchestrate::handlers::AuthorInput { name, sources },
                    )
                    .await
                }
                Err(err) => report(format, &err),
            },
            PlanAction::Execute => {
                run::<orchestrate::handlers::Execute, _, _>(
                    format,
                    p,
                    orchestrate::handlers::ExecuteInput {},
                )
                .await
            }
            PlanAction::Archive { force } => {
                run::<plan::handlers::Archive, _, _>(
                    format,
                    p,
                    plan::handlers::ArchiveInput { force },
                )
                .await
            }
        },
        Commands::Journal { action } => match action {
            JournalAction::Emit { event, payload } => {
                run::<journal::handlers::Emit, _, _>(
                    format,
                    p,
                    journal::handlers::EmitInput { event, payload },
                )
                .await
            }
            JournalAction::Show { filter, limit } => {
                run::<journal::handlers::Show, _, _>(
                    format,
                    p,
                    journal::handlers::ShowInput { filter, limit },
                )
                .await
            }
        },
        Commands::Registry { action } => match action {
            RegistryAction::Validate => {
                run::<registry::handlers::Validate, _, _>(
                    format,
                    p,
                    registry::handlers::ValidateInput {},
                )
                .await
            }
            RegistryAction::Add {
                name,
                url,
                adapter,
                description,
            } => {
                run::<registry::handlers::Add, _, _>(
                    format,
                    p,
                    registry::handlers::AddInput {
                        name,
                        url,
                        adapter,
                        description,
                    },
                )
                .await
            }
            RegistryAction::Remove { name } => {
                run::<registry::handlers::Remove, _, _>(
                    format,
                    p,
                    registry::handlers::RemoveInput { name },
                )
                .await
            }
        },
        Commands::Archive { action } => match action {
            ArchiveAction::Prune {
                keep,
                older_than,
                dry_run,
            } => {
                run::<slice::handlers::Prune, _, _>(
                    format,
                    p,
                    slice::handlers::PruneInput {
                        keep,
                        older_than,
                        dry_run,
                    },
                )
                .await
            }
        },
        // The scaffold leg only, matching the guest: the provisioning
        // half (hydration, manifest generation) stays with the shipped
        // path — a dev-shim-only implementation would fork the
        // operational surface off the wasm path.
        Commands::Init(args) if args.scaffold_only => {
            run::<init::handlers::Scaffold, _, _>(
                format,
                p,
                init::handlers::ScaffoldInput {
                    adapter: args.adapter,
                    name: args.name,
                    description: args.description,
                    workspace: args.workspace,
                    platforms: args.platforms,
                },
            )
            .await
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
