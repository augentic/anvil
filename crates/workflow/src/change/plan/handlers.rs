//! The `specify plan *` verb family plus the shared plan-file helpers.
//!
//! `plan author` / `plan execute` are orchestration verbs and live in
//! [`crate::orchestrate`].

mod add;
mod amend;
pub mod args;
mod create;
mod entry;
mod lifecycle;
mod remove;

use std::path::{Path, PathBuf};

use error::{Error, Result};
use serde::Serialize;

pub use self::add::{Add, AddInput};
pub use self::amend::{Amend, AmendInput};
pub use self::args::{BindingArg, KindAssign, SourceAssign, source_map};
pub use self::create::{Create, CreateInput};
pub use self::entry::EntryBody;
pub use self::lifecycle::{
    Archive, ArchiveInput, Next, NextInput, Status, StatusInput, Transition, TransitionInput,
    Validate, ValidateInput,
};
pub use self::remove::{Remove, RemoveInput};
use crate::change::Plan;
use crate::handler::Ctx;
use crate::registry::Registry;

// ---- Shared helpers used across submodules ----

/// Ensure the plan file exists before we try to load it. Error text is
/// the stable "plan file not found: plan.yaml" string that skill
/// authors match on. Resolves through `ctx.layout()` so the plan-root
/// override applies.
pub(crate) fn require_file(ctx: &Ctx) -> Result<PathBuf> {
    let path = ctx.layout().plan_path();
    if !path.exists() {
        return Err(Error::ArtifactNotFound {
            kind: "plan.yaml",
            path,
        });
    }
    Ok(path)
}

/// Name + path reference to the governing plan file, embedded in the
/// mutating verbs' response bodies.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Ref {
    /// Plan name from `plan.yaml.name`.
    pub name: String,
    /// Display path of the plan file.
    pub path: String,
}

pub(crate) fn plan_ref(plan: &Plan, plan_path: &Path) -> Ref {
    Ref {
        name: plan.name.to_string(),
        path: plan_path.display().to_string(),
    }
}

/// Verify that `project_name` appears in `registry.yaml`.
pub(crate) fn check_project(project_dir: &Path, project_name: &str) -> Result<()> {
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
