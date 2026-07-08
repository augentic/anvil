//! Dispatcher for the `specify plan *` verbs plus the shared plan-file helpers.
//!
//! `plan author` / `plan execute` are guest-owned collapsed orchestrations —
//! only their clap surface lives in `cli`.

mod add;
mod amend;
pub(crate) mod args;
pub mod cli;
mod create;
mod entry;
mod lifecycle;
mod remove;

use std::path::{Path, PathBuf};

use error::{Error, Result};
use serde::Serialize;
use workflow_lib::change::Plan;
use workflow_lib::registry::Registry;

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
        PlanAction::Remove { name } => remove::remove(ctx, name),
        PlanAction::Transition {
            name,
            target,
            undo,
            actor,
        } => lifecycle::transition(ctx, name, target, undo, &actor),
        PlanAction::Archive { force } => lifecycle::archive(ctx, force),
        // `plan execute` / `plan author` are guest-owned collapsed
        // orchestrations peeled off by both dispatchers before this
        // table (the native triage routes them to the guest leg; the
        // guest router routes them to `workflow_lib::orchestrate`).
        // The defensive arms keep the match exhaustive and never
        // collapse a real run to a misleading success.
        PlanAction::Execute => Err(Error::Argument {
            flag: "<command>",
            detail: "`specify plan execute` dispatches outside the shared verb table".to_string(),
        }),
        PlanAction::Author { .. } => Err(Error::Argument {
            flag: "<command>",
            detail: "`specify plan author` dispatches outside the shared verb table".to_string(),
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
