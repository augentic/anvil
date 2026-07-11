//! `plan archive` — move the current plan into the archive.

use std::io::Write;
use std::path::PathBuf;

use error::Error;
use omnia_guest::api::invoke::CallContext;
use omnia_guest::api::operation::Operation;
use serde::{Deserialize, Serialize};

use crate::change::Plan;
use crate::handler::{Anchor, Ctx, Render};

/// Wire input for `plan archive`.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ArchiveInput {
    /// Archive even when the plan has pending or in-progress entries.
    #[serde(default)]
    pub force: bool,
}

/// `specify plan archive` — move the current plan to
/// `.specify/archive/plans/<name>-<YYYYMMDD>.yaml`.
#[derive(Clone, Copy, Debug)]
pub struct Archive;

impl<P: Anchor> Operation<P> for Archive {
    type Error = crate::handler::Error;
    type Input = ArchiveInput;
    type Output = ArchiveBody;

    async fn call(
        input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let cx = Ctx::load(context.provider)?;
        let layout = cx.layout();
        let plan_path = layout.plan_path();
        if !plan_path.exists() {
            return Err(Error::ArtifactNotFound {
                kind: "plan.yaml",
                path: plan_path,
            }
            .into());
        }
        let archive_dir = layout.archive_dir().join("plans");
        let brief_path = layout.change_brief_path();
        let plan_name = Plan::load(&plan_path)?.name.into_string();

        let (archived, archived_plans_dir) =
            Plan::archive(&plan_path, &brief_path, &archive_dir, input.force, cx.now())?;
        Ok(ArchiveBody {
            archived,
            archived_plans_dir,
            plan: ArchivedPlan { name: plan_name },
        })
    }
}

/// Success envelope for `plan archive`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ArchiveBody {
    /// Path of the archived plan file (serialised as its display string).
    pub archived: PathBuf,
    /// Path of the moved working directory, when one moved.
    pub archived_plans_dir: Option<PathBuf>,
    /// The archived plan's identity.
    pub plan: ArchivedPlan,
}

/// The archived plan's identity.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ArchivedPlan {
    /// Plan name.
    pub name: String,
}

impl Render for ArchiveBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        match &self.archived_plans_dir {
            Some(dir) => writeln!(
                w,
                "Archived plan to {}. Working directory moved to {}.",
                self.archived.display(),
                dir.display()
            ),
            None => writeln!(w, "Archived plan to {}.", self.archived.display()),
        }
    }
}
