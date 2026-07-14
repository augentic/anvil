//! Slice lifecycle verbs: drop. Create and transition are internal —
//! the refine / build / merge orchestrations drive the crate-private
//! `slice::actions` kernels directly.

use std::io::Write;
use std::path::PathBuf;

use omnia_guest::api::invoke::CallContext;
use omnia_guest::api::operation::Operation;
use project::handler::{Anchor, Ctx, Render};
use serde::{Deserialize, Serialize};

use crate::{LifecycleStatus, actions as slice_actions};

/// Wire input for `slice drop`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct DropInput {
    /// Slice to drop.
    pub name: String,
    /// Free-text reason; surfaced in `metadata.yaml.drop_reason` and
    /// the archive path.
    #[serde(default)]
    pub reason: Option<String>,
}

/// `specify slice drop <name>` — transition a slice to `dropped` and
/// archive it.
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
        let slice_dir = cx.layout().slice_dir(&name);
        let archive_dir = cx.layout().archive_dir();
        let (metadata, archive_path) =
            slice_actions::discard(&slice_dir, &archive_dir, reason.as_deref(), cx.now())?;
        Ok(DropBody {
            name,
            status: metadata.status,
            archive_path,
            drop_reason: metadata.drop_reason,
        })
    }
}

/// Success envelope for `slice drop`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct DropBody {
    /// Dropped slice.
    pub name: String,
    /// Persisted lifecycle state.
    pub status: LifecycleStatus,
    /// Archived slice location.
    pub archive_path: PathBuf,
    /// Persisted reason, when supplied.
    pub drop_reason: Option<String>,
}

impl Render for DropBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        writeln!(w, "{}: dropped and archived to {}", self.name, self.archive_path.display())?;
        if let Some(r) = &self.drop_reason {
            writeln!(w, "  reason: {r}")?;
        }
        Ok(())
    }
}
