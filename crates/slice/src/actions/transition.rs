//! Phase stamp helper: record `*_at` timestamps and refine facts
//! (RFC-86 D2 / D11). Lifecycle labels are projected, never stored.

use std::path::Path;

use error::Error;
use jiff::Timestamp;
use project::config::Layout;
use project::journal::{Event, EventKind, append_one};

use crate::{LifecycleStatus, SLICES_DIR_NAME, SliceMetadata};

/// Stamp the phase timestamp for `target` and, for `Refined`, append
/// `slice.transition.refined`.
///
/// `target` selects which timestamp to stamp — it is not persisted as
/// a status field. Progress projects from artifacts and facts
/// (RFC-86 D2). Timestamps remain for operators and for drop detection
/// (`dropped_at`).
///
/// Returns the updated `SliceMetadata`.
///
/// # Errors
///
/// Propagates load / save failures from `SliceMetadata`.
pub fn transition(
    slice_dir: &Path, target: LifecycleStatus, now: Timestamp,
) -> Result<SliceMetadata, Error> {
    let mut metadata = SliceMetadata::load(slice_dir)?;
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
