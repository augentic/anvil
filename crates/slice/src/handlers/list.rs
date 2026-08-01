//! `slice list` — read-only listing of every slice under
//! `.emery/slices/` with its lifecycle status.

use std::io::Write;

use omnia_guest::api::invoke::CallContext;
use omnia_guest::api::operation::Operation;
use project::handler::{Anchor, Ctx, Render};
use serde::{Deserialize, Serialize};

use crate::{LifecycleStatus, SliceMetadata};

/// Wire input for `slice list` (no fields).
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[expect(
    clippy::empty_structs_with_brackets,
    reason = "serde deserialises the wire `{}` object into a braced struct only"
)]
pub struct ListInput {}

/// `emery slice list`.
#[derive(Clone, Copy, Debug)]
pub struct List;

impl<P: Anchor> Operation<P> for List {
    type Error = project::handler::Error;
    type Input = ListInput;
    type Output = ListBody;

    async fn call(
        _input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let cx = Ctx::load(context.provider)?;
        let slices_dir = cx.layout().slices_dir();
        let mut slices = Vec::new();
        if slices_dir.is_dir() {
            for entry in std::fs::read_dir(&slices_dir).map_err(error::Error::Io)? {
                let dir = entry.map_err(error::Error::Io)?.path();
                if !SliceMetadata::path(&dir).is_file() {
                    continue;
                }
                let metadata = SliceMetadata::load(&dir)?;
                let name = dir.file_name().unwrap_or_default().to_string_lossy().into_owned();
                slices.push(ListEntry {
                    name,
                    status: metadata.status,
                    target: metadata.target,
                });
            }
        }
        slices.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(ListBody { slices })
    }
}

/// Success envelope for `slice list`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ListBody {
    /// Every slice under `.emery/slices/`, sorted by name.
    pub slices: Vec<ListEntry>,
}

/// One listed slice.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ListEntry {
    /// Slice directory name.
    pub name: String,
    /// Lifecycle status from `metadata.yaml`.
    pub status: LifecycleStatus,
    /// Recorded target value (e.g. `omnia@1.0.0`).
    pub target: String,
}

impl Render for ListBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        if self.slices.is_empty() {
            writeln!(w, "no slices")?;
            return Ok(());
        }
        for entry in &self.slices {
            writeln!(w, "{}  {}  {}", entry.name, entry.status, entry.target)?;
        }
        Ok(())
    }
}
