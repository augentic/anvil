//! `transition` verb: validate a lifecycle edge and stamp the matching
//! `*_at` timestamp.

use std::path::Path;

use error::Error;
use jiff::Timestamp;
use project::config::Layout;
use project::journal::{Event, EventKind, append_one};

use crate::{LifecycleStatus, SLICES_DIR_NAME, SliceMetadata};

/// Transition a slice to `target` status and write the matching timestamp.
///
/// The transition is validated by
/// [`LifecycleStatus::transition`](crate::LifecycleStatus::transition) —
/// illegal edges return `Error::Diag` with `code = "lifecycle"` without
/// touching disk. On success the metadata's `status` is updated, the
/// appropriate `*_at` timestamp is filled in (idempotent: an existing
/// non-`None` timestamp is preserved), and `metadata.yaml` is rewritten
/// atomically.
///
/// Returns the updated `SliceMetadata`.
///
/// # Errors
///
/// `Error::Diag` with `code = "lifecycle"` for an illegal edge; otherwise
/// propagates load / save failures from `SliceMetadata`.
pub fn transition(
    slice_dir: &Path, target: LifecycleStatus, now: Timestamp,
) -> Result<SliceMetadata, Error> {
    let mut metadata = SliceMetadata::load(slice_dir)?;
    metadata.status = metadata.status.transition(target)?;
    let stamp = now;
    match target {
        LifecycleStatus::Refining => {
            if metadata.created_at.is_none() {
                metadata.created_at = Some(stamp);
            }
        }
        LifecycleStatus::Refined => {
            if metadata.defined_at.is_none() {
                metadata.defined_at = Some(stamp);
            }
        }
        LifecycleStatus::Built => {
            if metadata.completed_at.is_none() {
                metadata.completed_at = Some(stamp);
            }
        }
        LifecycleStatus::Merged => {
            if metadata.merged_at.is_none() {
                metadata.merged_at = Some(stamp);
            }
        }
        LifecycleStatus::Dropped => {
            if metadata.dropped_at.is_none() {
                metadata.dropped_at = Some(stamp);
            }
        }
    }
    metadata.save(slice_dir)?;

    if target == LifecycleStatus::Refined {
        let slice_name =
            slice_dir.file_name().and_then(|s| s.to_str()).unwrap_or("unknown").to_string();
        if let Some(project_root) = project_root(slice_dir) {
            let event = Event::new(
                now,
                EventKind::SliceTransitionRefined {
                    slice_name: slice_name.into(),
                },
            );
            append_one(Layout::new(&project_root), &event)?;
        }
    }

    Ok(metadata)
}

/// Resolve the project root from `<project>/.emery/slices/<name>/`.
fn project_root(slice_path: &Path) -> Option<std::path::PathBuf> {
    let slices_parent = slice_path.parent()?;
    if slices_parent.file_name()? != std::ffi::OsStr::new(SLICES_DIR_NAME) {
        return None;
    }
    let emery_dir = slices_parent.parent()?;
    emery_dir.parent().map(Path::to_path_buf)
}
