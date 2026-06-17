mod add;
mod amend;
mod args;
pub mod cli;
mod create;
mod entry;
mod lifecycle;
pub mod lock;
mod propose;
mod remove;

use std::path::{Path, PathBuf};

use serde::Serialize;
use specify_error::{Error, Result};
use specify_workflow::change::Plan;
use specify_workflow::registry::Registry;

use self::cli::PlanAction;
use crate::runtime::context::Ctx;

pub fn run(ctx: &Ctx, action: PlanAction) -> Result<()> {
    match action {
        PlanAction::Create {
            name,
            sources,
            auto_approve,
            authority_override,
        } => create::create(ctx, name, sources, auto_approve, &authority_override),
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
        // `plan lock` is peeled off by the dispatcher (it passes the
        // child's exit code through `Exit::Code`); this defensive arm
        // keeps the match exhaustive and never collapses a real run to
        // a misleading success.
        PlanAction::Lock { command } => lock::run(ctx, &command).map(|_code| ()),
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
