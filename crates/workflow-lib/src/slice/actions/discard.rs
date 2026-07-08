//! `discard` verb: transition to `Dropped`, stamp the reason, archive.

use std::path::{Path, PathBuf};

use jiff::Timestamp;
use specify_error::Error;

use super::archive::archive;
use crate::slice::{LifecycleStatus, SliceMetadata};

/// Transition a slice to `Dropped`, record the optional reason, then
/// archive. Returns the final archive path.
///
/// Valid from any non-terminal lifecycle state. Callers use this for
/// both failure ("dropped because build broke") and deferral ("blocked
/// on a design question") — the plan layer above turns the reason into
/// `failure-reason` or `block-reason`; here it's just free text.
///
/// # Errors
///
/// `Error::Diag` with `code = "lifecycle"` if the slice is already
/// terminal; otherwise propagates whatever `transition` and `archive`
/// surface.
pub fn discard(
    slice_dir: &Path, archive_dir: &Path, reason: Option<&str>, now: Timestamp,
) -> Result<(SliceMetadata, PathBuf), Error> {
    let mut metadata = SliceMetadata::load(slice_dir)?;
    metadata.status = metadata.status.transition(LifecycleStatus::Dropped)?;
    metadata.dropped_at = Some(now);
    if let Some(text) = reason {
        metadata.drop_reason = Some(text.to_string());
    }
    metadata.save(slice_dir)?;
    let target = archive(slice_dir, archive_dir, now)?;
    Ok((metadata, target))
}
