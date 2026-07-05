//! Dispatcher for the `specify plan *` verbs (minus `plan lock`, whose
//! subprocess-spawning handler lives in the binary crate) plus the
//! shared plan-file helpers its submodules use.

mod add;
mod amend;
pub(crate) mod args;
pub mod cli;
mod create;
mod entry;
mod lifecycle;
mod propose;
mod remove;

use std::path::{Path, PathBuf};

use serde::Serialize;
use specify_error::{Error, Result};
use specify_workflow::change::Plan;
use specify_workflow::registry::Registry;

use self::cli::PlanAction;
use crate::context::Ctx;

/// Dispatch one parsed `specify plan` action against the loaded `ctx`.
///
/// # Errors
///
/// Propagates the invoked handler's failure.
pub fn run(ctx: &Ctx, action: PlanAction) -> Result<()> {
    match action {
        PlanAction::Create {
            name,
            sources,
            intent,
            auto_approve,
            authority_override,
        } => create::create(ctx, name, sources, intent, auto_approve, &authority_override),
        PlanAction::Validate => lifecycle::validate(ctx),
        PlanAction::Next => lifecycle::next(ctx),
        PlanAction::Status => lifecycle::status(ctx),
        PlanAction::Add(args) => add::add(ctx, args),
        PlanAction::Amend(args) => amend::amend(ctx, args),
        PlanAction::Propose(args) => propose::propose(ctx, args),
        PlanAction::Remove { name } => remove::remove(ctx, name),
        PlanAction::Transition {
            name,
            target,
            undo,
            actor,
        } => lifecycle::transition(ctx, name, target, undo, &actor),
        PlanAction::Archive { force } => lifecycle::archive(ctx, force),
        // `plan lock` is peeled off by both dispatchers before this
        // table (the native binary passes the child's exit code through
        // `Exit::Code`; the guest refuses it — no subprocesses
        // in-guest). This defensive arm keeps the match exhaustive and
        // never collapses a real run to a misleading success.
        PlanAction::Lock { .. } => Err(Error::Argument {
            flag: "<command>",
            detail: "`specify plan lock` dispatches outside the shared verb table".to_string(),
        }),
        // Guest-only: the guest router peels `plan execute` off into an
        // orchestration before this table, so reaching this arm means
        // the native binary parsed it — refuse with the mirror image of
        // the guest's native-only refusals (wire code `argument`,
        // exit 2). Natively the loop is skill-owned until Step 5.
        PlanAction::Execute => Err(Error::Argument {
            flag: "<command>",
            detail: "`specify plan execute` runs only in the workflow guest; natively the \
                     execute loop is driven by the /spec:execute skill"
                .to_string(),
        }),
        // Guest-only, same posture as `plan execute`: the collapsed
        // plan-authoring flow is an orchestration; natively the flow is
        // driven by the /spec:plan skill through the per-verb surface.
        PlanAction::Author { .. } => Err(Error::Argument {
            flag: "<command>",
            detail: "`specify plan author` runs only in the workflow guest; natively \
                     plan authoring is driven by the /spec:plan skill"
                .to_string(),
        }),
    }
}

// ---- Shared helpers used across submodules ----

/// Ensure the plan file exists before we try to load it. Error text is
/// the stable "plan file not found: plan.yaml" string that skill
/// authors match on. Resolves through `ctx.layout()` so the global
/// `--plan-dir` plan-root override applies.
pub(super) fn require_file(ctx: &Ctx) -> Result<PathBuf> {
    let path = ctx.layout().plan_path();
    if !path.exists() {
        return Err(Error::ArtifactNotFound {
            kind: "plan.yaml",
            path,
        });
    }
    Ok(path)
}

#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
pub(super) struct Ref {
    pub name: String,
    pub path: String,
}

pub(super) fn plan_ref(plan: &Plan, plan_path: &Path) -> Ref {
    Ref {
        name: plan.name.to_string(),
        path: plan_path.display().to_string(),
    }
}

/// Verify that `project_name` appears in `registry.yaml`.
pub(super) fn check_project(project_dir: &Path, project_name: &str) -> Result<()> {
    match Registry::load(project_dir) {
        Ok(Some(registry)) => {
            if !registry.projects.iter().any(|p| p.name == project_name) {
                return Err(Error::Diag {
                    code: "plan-project-unknown",
                    detail: format!(
                        "--project '{project_name}' does not match any project in registry.yaml"
                    ),
                });
            }
            Ok(())
        }
        Ok(None) => Err(Error::Diag {
            code: "plan-project-no-registry",
            detail: "--project was specified but no registry.yaml exists".to_string(),
        }),
        Err(err) => Err(err),
    }
}
