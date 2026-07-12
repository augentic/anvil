//! The `specify plan *` verb family plus the shared plan-file helpers.

mod add;
mod amend;
mod archive;
mod author;
mod create;
mod entry;
mod execute;
mod next;
mod remove;
mod status;
mod transition;
mod validate;

use std::path::{Path, PathBuf};

use error::{Error, Result};
use serde::Serialize;

pub use self::add::{Add, AddInput};
pub use self::amend::{Amend, AmendInput};
pub use self::archive::{Archive, ArchiveBody, ArchiveInput, ArchivedPlan};
pub use self::author::{Author, AuthorBody, AuthorInput, AuthorSurvey};
pub use self::create::{Create, CreateInput};
pub use self::entry::EntryBody;
pub use self::execute::{Execute, ExecuteBody, ExecuteInput, ExecutePhase};
pub use self::next::{Next, NextInput};
pub use self::remove::{Remove, RemoveInput};
pub use self::status::{Status, StatusInput};
pub use self::transition::{Transition, TransitionBody, TransitionInput, TransitionKind, UndoPair};
pub use self::validate::{Validate, ValidateInput};
use crate::change::Plan;
use crate::handler::Ctx;
use crate::registry::Registry;

// ---- Shared helpers used across submodules ----

/// Ensure the plan file exists before we try to load it. Error text is
/// the stable "plan file not found: plan.yaml" string that skill
/// authors match on.
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
    /// Path of the plan file (serialised as its display string).
    pub path: PathBuf,
}

pub(crate) fn plan_ref(plan: &Plan, plan_path: &Path) -> Ref {
    Ref {
        name: plan.name.to_string(),
        path: plan_path.to_path_buf(),
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
