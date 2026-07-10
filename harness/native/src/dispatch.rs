//! The native shim's exhaustive dispatch match over [`Commands`].
//!
//! One arm per leaf verb, converting the parsed clap action into the
//! matching verb input DTO and driving the handler through
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
/// carry real reference grants), and drive the matched verb to its
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

/// Route one parsed invocation: convert the clap action to the verb's
/// input DTO and run the handler; refuse the provisioning verbs.
#[expect(clippy::too_many_lines, reason = "one arm per grammar leaf, exhaustive by design")]
async fn dispatch(cli: Cli, p: &NativeProvider<DevModel>) -> Exit {
    let format = cli.format;
    if let Err(err) = check_plan_dir(cli.plan_dir.as_deref()) {
        return report(format, &err);
    }
    match cli.command {
        Commands::Source { action } => match action {
            SourceAction::Resolve { name, project_dir } => {
                run::<adapter::verbs::SourceResolve, _, _>(
                    format,
                    p,
                    adapter::verbs::ResolveInput {
                        value: name,
                        project_dir: Some(project_dir),
                    },
                )
                .await
            }
            SourceAction::Survey { source, plan } => {
                run::<orchestrate::verbs::Survey, _, _>(
                    format,
                    p,
                    orchestrate::verbs::SurveyInput { source, plan },
                )
                .await
            }
            SourceAction::Extract { source, lead, slice } => {
                run::<orchestrate::verbs::Extract, _, _>(
                    format,
                    p,
                    orchestrate::verbs::ExtractInput { source, lead, slice },
                )
                .await
            }
        },
        Commands::Target { action } => match action {
            TargetAction::Resolve { value, project_dir } => {
                run::<adapter::verbs::TargetResolve, _, _>(
                    format,
                    p,
                    adapter::verbs::ResolveInput {
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
                run::<slice::verbs::Create, _, _>(
                    format,
                    p,
                    slice::verbs::CreateInput {
                        name,
                        target,
                        if_exists: if_exists.to_string(),
                    },
                )
                .await
            }
            SliceAction::Validate { name } => {
                run::<slice::verbs::Validate, _, _>(format, p, slice::verbs::ValidateInput { name })
                    .await
            }
            SliceAction::Provenance { name } => {
                run::<slice::verbs::Provenance, _, _>(
                    format,
                    p,
                    slice::verbs::ProvenanceInput { name },
                )
                .await
            }
            SliceAction::Model { action } => match action {
                SliceModelAction::Show { name } => {
                    run::<slice::verbs::ModelShow, _, _>(
                        format,
                        p,
                        slice::verbs::ModelShowInput { name },
                    )
                    .await
                }
            },
            SliceAction::Refine { name } => {
                run::<orchestrate::verbs::Refine, _, _>(
                    format,
                    p,
                    orchestrate::verbs::RefineInput { name },
                )
                .await
            }
            SliceAction::Build { name } => {
                run::<orchestrate::verbs::Build, _, _>(
                    format,
                    p,
                    orchestrate::verbs::BuildInput { name },
                )
                .await
            }
            SliceAction::Merge { action } => match action {
                SliceMergeAction::Run {
                    name,
                    allow_composition_replace,
                } => {
                    run::<orchestrate::verbs::MergeRun, _, _>(
                        format,
                        p,
                        orchestrate::verbs::MergeRunInput {
                            name,
                            allow_composition_replace,
                        },
                    )
                    .await
                }
                SliceMergeAction::Preview { name } => {
                    run::<slice::verbs::Preview, _, _>(
                        format,
                        p,
                        slice::verbs::PreviewInput { name },
                    )
                    .await
                }
                SliceMergeAction::ConflictCheck { name } => {
                    run::<slice::verbs::ConflictCheck, _, _>(
                        format,
                        p,
                        slice::verbs::ConflictCheckInput { name },
                    )
                    .await
                }
            },
            SliceAction::Task { action } => match action {
                SliceTaskAction::Progress { name } => {
                    run::<slice::verbs::TaskProgress, _, _>(
                        format,
                        p,
                        slice::verbs::TaskProgressInput { name },
                    )
                    .await
                }
                SliceTaskAction::Mark { name, task_number } => {
                    run::<slice::verbs::TaskMark, _, _>(
                        format,
                        p,
                        slice::verbs::TaskMarkInput { name, task_number },
                    )
                    .await
                }
            },
            SliceAction::Transition { name, target } => {
                run::<slice::verbs::Transition, _, _>(
                    format,
                    p,
                    slice::verbs::TransitionInput { name, target },
                )
                .await
            }
            SliceAction::TouchedSpecs { name, scan, set } => {
                run::<slice::verbs::TouchedSpecs, _, _>(
                    format,
                    p,
                    slice::verbs::TouchedSpecsInput { name, scan, set },
                )
                .await
            }
            SliceAction::Overlap { name } => {
                run::<slice::verbs::Overlap, _, _>(format, p, slice::verbs::OverlapInput { name })
                    .await
            }
            SliceAction::Drop { name, reason } => {
                run::<slice::verbs::Drop, _, _>(format, p, slice::verbs::DropInput { name, reason })
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
                    run::<plan::verbs::Create, _, _>(
                        format,
                        p,
                        plan::verbs::CreateInput {
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
                run::<plan::verbs::Validate, _, _>(format, p, plan::verbs::ValidateInput {}).await
            }
            PlanAction::Next => {
                run::<plan::verbs::Next, _, _>(format, p, plan::verbs::NextInput {}).await
            }
            PlanAction::Status => {
                run::<plan::verbs::Status, _, _>(format, p, plan::verbs::StatusInput {}).await
            }
            PlanAction::Add(args) => {
                run::<plan::verbs::Add, _, _>(
                    format,
                    p,
                    plan::verbs::AddInput {
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
                run::<plan::verbs::Amend, _, _>(
                    format,
                    p,
                    plan::verbs::AmendInput {
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
                run::<plan::verbs::Remove, _, _>(format, p, plan::verbs::RemoveInput { name }).await
            }
            PlanAction::Transition {
                name,
                target,
                undo,
                actor,
            } => {
                run::<plan::verbs::Transition, _, _>(
                    format,
                    p,
                    plan::verbs::TransitionInput {
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
                    run::<orchestrate::verbs::Author, _, _>(
                        format,
                        p,
                        orchestrate::verbs::AuthorInput { name, sources },
                    )
                    .await
                }
                Err(err) => report(format, &err),
            },
            PlanAction::Execute => {
                run::<orchestrate::verbs::Execute, _, _>(
                    format,
                    p,
                    orchestrate::verbs::ExecuteInput {},
                )
                .await
            }
            PlanAction::Archive { force } => {
                run::<plan::verbs::Archive, _, _>(format, p, plan::verbs::ArchiveInput { force })
                    .await
            }
        },
        Commands::Journal { action } => match action {
            JournalAction::Emit { event, payload } => {
                run::<journal::verbs::Emit, _, _>(
                    format,
                    p,
                    journal::verbs::EmitInput { event, payload },
                )
                .await
            }
            JournalAction::Show { filter, limit } => {
                run::<journal::verbs::Show, _, _>(
                    format,
                    p,
                    journal::verbs::ShowInput { filter, limit },
                )
                .await
            }
        },
        Commands::Registry { action } => match action {
            RegistryAction::Validate => {
                run::<registry::verbs::Validate, _, _>(format, p, registry::verbs::ValidateInput {})
                    .await
            }
            RegistryAction::Add {
                name,
                url,
                adapter,
                description,
            } => {
                run::<registry::verbs::Add, _, _>(
                    format,
                    p,
                    registry::verbs::AddInput {
                        name,
                        url,
                        adapter,
                        description,
                    },
                )
                .await
            }
            RegistryAction::Remove { name } => {
                run::<registry::verbs::Remove, _, _>(
                    format,
                    p,
                    registry::verbs::RemoveInput { name },
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
                run::<slice::verbs::Prune, _, _>(
                    format,
                    p,
                    slice::verbs::PruneInput {
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
            run::<init::verbs::Scaffold, _, _>(
                format,
                p,
                init::verbs::ScaffoldInput {
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

/// Refuse a provisioning verb on the standard argument-error surface
/// (wire code `argument`, exit 2) — the same set the guest refuses.
fn unsupported(format: Format, verb: &'static str) -> Exit {
    report(
        format,
        &Error::Argument {
            flag: "<command>",
            detail: format!("`specify {verb}` has no native dev-shim implementation"),
        },
    )
}
