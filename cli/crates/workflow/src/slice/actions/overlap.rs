//! `overlap` verb plus the [`Overlap`] DTO it returns.

use std::path::Path;

use serde::Serialize;
use specify_error::Error;

use crate::slice::{LifecycleStatus, SliceMetadata, SpecKind};

/// A capability-level conflict between two active slices both touching
/// the same spec.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Overlap {
    /// The shared capability name.
    pub capability: String,
    /// Name of the other slice that touches the same capability.
    pub other_slice: String,
    /// How our slice touches the capability.
    pub our_spec_type: SpecKind,
    /// How the other slice touches the capability.
    pub other_spec_type: SpecKind,
}

/// Detect overlap between this slice's `touched_specs` and every other
/// active slice's. "Active" means a directory under `slices_dir` that
/// has a `metadata.yaml` and is not `slice_name` itself.
///
/// Merged and dropped slices still appear on disk until the archive
/// move completes, so we additionally filter by status: only
/// non-terminal statuses participate. Archive directories under
/// `slices_dir` (e.g. `<slices_dir>/archive/...`) are not scanned.
///
/// # Errors
///
/// Propagates load failures from `SliceMetadata::load` for either the
/// caller's slice or any sibling, plus any I/O error from walking
/// `slices_dir` entries.
pub fn overlap(slices_dir: &Path, slice_name: &str) -> Result<Vec<Overlap>, Error> {
    let self_dir = slices_dir.join(slice_name);
    let self_meta = SliceMetadata::load(&self_dir)?;
    if self_meta.touched_specs.is_empty() {
        return Ok(Vec::new());
    }

    let mut overlaps: Vec<Overlap> = Vec::new();
    let Ok(entries) = std::fs::read_dir(slices_dir) else {
        return Ok(Vec::new());
    };
    for entry in entries {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let other_path = entry.path();
        let Some(other_name) = other_path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if other_name == slice_name || other_name == "archive" {
            continue;
        }
        if !SliceMetadata::path(&other_path).exists() {
            continue;
        }
        let other_meta = SliceMetadata::load(&other_path)?;
        if matches!(other_meta.status, LifecycleStatus::Merged | LifecycleStatus::Dropped) {
            continue;
        }
        for ours in &self_meta.touched_specs {
            for theirs in &other_meta.touched_specs {
                if ours.name == theirs.name {
                    overlaps.push(Overlap {
                        capability: ours.name.clone(),
                        other_slice: other_name.to_string(),
                        our_spec_type: ours.kind,
                        other_spec_type: theirs.kind,
                    });
                }
            }
        }
    }
    overlaps.sort_by(|a, b| {
        a.capability.cmp(&b.capability).then_with(|| a.other_slice.cmp(&b.other_slice))
    });
    Ok(overlaps)
}
