//! `plan archive` — move the current plan into the archive and run
//! the change-scoped snapshot sweep.

use std::io::Write;
use std::path::{Path, PathBuf};

use error::Error;
use omnia_guest::api::invoke::CallContext;
use omnia_guest::api::operation::Operation;
use project::build_record::BuildRecord;
use project::handler::{Anchor, Ctx, Render};
use project::plan::Plan;
use project::seam::Workspaces;
use project::snapshot::SnapshotId;
use serde::{Deserialize, Serialize};
use slice::refinement::Manifest;

/// Wire input for `plan archive`.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ArchiveInput {
    /// Archive even when the plan has pending or in-progress entries.
    #[serde(default)]
    pub force: bool,
}

/// `emery plan archive` — close the change.
///
/// Moves the current plan to
/// `.emery/archive/plans/<name>-<YYYYMMDD>.yaml`, then sweeps the
/// snapshot store: the archived change's pins stop being GC roots
/// (RFC-88 D2), so objects reachable only from archived slice trees
/// are deleted.
#[derive(Clone, Copy, Debug)]
pub struct Archive;

impl<P: Anchor + Workspaces> Operation<P> for Archive {
    type Error = project::handler::Error;
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

        // Change-scoped collection: pins under the archive tree are
        // dead roots; pins under any slice tree still live (a forced
        // archive leaves unfinished slices in place) are kept.
        let dead = collect_pins(&layout.archive_dir())?;
        let live = collect_pins(&layout.slices_dir())?;
        let swept_objects =
            context.provider.sweep(dead, live).await.map_err(|err| Error::Diag {
                code: "snapshot-sweep-failed",
                detail: format!(
                    "plan `{plan_name}` archived, but the snapshot sweep failed: {err}"
                ),
            })?;

        Ok(ArchiveBody {
            archived,
            archived_plans_dir,
            swept_objects,
            plan: ArchivedPlan { name: plan_name },
        })
    }
}

/// Every snapshot pin recorded beneath one level of slice-shaped
/// directories under `root`: refinement-manifest input pins plus each
/// `builds/<digest>.yaml` record's base and result snapshots. Roots
/// that never reached the store are skipped by the sweep itself.
fn collect_pins(root: &Path) -> Result<Vec<SnapshotId>, Error> {
    let mut pins = Vec::new();
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(pins),
        Err(source) => {
            return Err(Error::Filesystem {
                op: "read_dir",
                path: root.to_path_buf(),
                source,
            });
        }
    };
    for entry in entries.filter_map(Result::ok) {
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        if Manifest::path(&dir).is_file() {
            let manifest = Manifest::load(&dir)?;
            pins.extend(manifest.inputs.sources.into_values());
            pins.push(manifest.inputs.baseline_specs);
        }
        for record in BuildRecord::load_all(&dir)? {
            pins.push(record.base);
            pins.push(record.result);
        }
    }
    Ok(pins)
}

/// Success envelope for `plan archive`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ArchiveBody {
    /// Path of the archived plan file (serialised as its display string).
    pub archived: PathBuf,
    /// Path of the moved working directory, when one moved.
    pub archived_plans_dir: Option<PathBuf>,
    /// Snapshot objects deleted by the change-scoped sweep.
    pub swept_objects: usize,
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
        writeln!(w, "archived plan `{}`", self.plan.name)?;
        writeln!(w, "  archived: {}", self.archived.display())?;
        if let Some(dir) = &self.archived_plans_dir {
            writeln!(w, "  working directory: {}", dir.display())?;
        }
        writeln!(w, "  swept snapshot objects: {}", self.swept_objects)?;
        Ok(())
    }
}
