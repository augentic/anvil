//! Abandon one plan entry's slice without merging.
//!
//! Stamps `dropped_at` and archives the slice tree; the entry stays on
//! the plan (in-scope exclusion) and the decomposition is unchanged.

use std::io::Write;
use std::path::PathBuf;

use error::Error;
use omnia_guest::api::invoke::CallContext;
use omnia_guest::api::operation::Operation;
use project::handler::{Anchor, Ctx, Render};
use project::plan::Plan;
use serde::{Deserialize, Serialize};

use super::require_file;

/// Wire input for `plan drop`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct DropInput {
    /// Plan entry (slice) to drop.
    pub name: String,
    /// Free-text reason; surfaced in `metadata.yaml.drop_reason` and
    /// the archive path.
    #[serde(default)]
    pub reason: Option<String>,
}

/// `emery plan drop <entry> [--reason]` — stamp the entry's slice
/// `dropped` and archive it.
#[derive(Clone, Copy, Debug)]
pub struct Drop;

impl<P: Anchor> Operation<P> for Drop {
    type Error = project::handler::Error;
    type Input = DropInput;
    type Output = DropBody;

    async fn call(
        input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let cx = Ctx::load(context.provider)?;
        let DropInput { name, reason } = input;
        let plan_path = require_file(&cx)?;
        let plan = Plan::load(&plan_path)?;
        if !plan.entries.iter().any(|entry| entry.name == name) {
            return Err(plan.entry_not_found(&name).into());
        }
        let slice_dir = cx.layout().slice_dir(&name);
        if !slice_dir.is_dir() {
            return Err(Error::Diag {
                code: "plan-drop-no-slice",
                detail: format!(
                    "plan entry `{name}` has no slice tree at {} — a never-refined entry is \
                     curated with `emery plan remove {name}` instead",
                    slice_dir.display()
                ),
            }
            .into());
        }
        let archive_dir = cx.layout().archive_dir();
        let (metadata, archive_path) =
            slice::discard(&slice_dir, &archive_dir, reason.as_deref(), cx.now())?;
        Ok(DropBody {
            name,
            archive_path,
            drop_reason: metadata.drop_reason,
        })
    }
}

/// Success envelope for `plan drop`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct DropBody {
    /// Dropped plan entry (slice).
    pub name: String,
    /// Archived slice location.
    pub archive_path: PathBuf,
    /// Persisted reason, when supplied.
    pub drop_reason: Option<String>,
}

impl Render for DropBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        writeln!(w, "dropped `{}`", self.name)?;
        writeln!(w, "  archived: {}", self.archive_path.display())?;
        if let Some(reason) = &self.drop_reason {
            writeln!(w, "  reason: {reason}")?;
        }
        Ok(())
    }
}
