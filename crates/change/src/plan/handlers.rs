//! The `emery plan *` verb family plus the shared plan-file helpers.

mod add;
mod amend;
mod archive;
mod author;
mod defer;
mod drop;
mod entry;
mod execute;
mod gaps;
mod remove;
mod status;
mod validate;

use std::path::{Path, PathBuf};

use error::{Error, Result};
use project::handler::Ctx;
use project::plan::Plan;
use serde::Serialize;

pub use self::add::{Add, AddInput};
pub use self::amend::{Amend, AmendInput};
pub use self::archive::{Archive, ArchiveBody, ArchiveInput, ArchivedPlan};
pub use self::author::{Author, AuthorBody, AuthorInput, AuthorSurvey};
pub use self::defer::{Defer, DeferAction, DeferBody, DeferInput, DeferSelector, DeferredGap};
pub use self::drop::{Drop, DropBody, DropInput};
pub use self::entry::EntryBody;
pub use self::execute::{Execute, ExecuteBody, ExecuteInput, ExecutePhase};
pub use self::gaps::{Gaps, GapsInput};
pub use self::remove::{Remove, RemoveInput};
pub use self::status::{Status, StatusInput};
pub use self::validate::{Validate, ValidateInput};

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
