//! Commit side of the slice-merge engine: atomic baseline writes for
//! 3-way classes, opaque-replace whole-file copy, and the
//! summary-string builder stamped into `metadata.yaml.outcome`.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use specify_error::Error;
use specify_model::atomic::bytes_write;

use super::MergePreviewEntry;
use crate::merge::artifact_class::{ArtifactClass, MergeStrategy};

/// Write each merged baseline produced by [`super::read::plan_three_way`]
/// to its target path via [`bytes_write`], which creates parent
/// directories and persists each file with a temp-file + rename so a
/// reader never observes a torn baseline. Caller guarantees every entry
/// has already validated.
///
/// Atomicity is per file, not across the set: a crash mid-loop can leave
/// some baselines updated and others not. That is safe because the caller
/// flips the slice to `Merged` only after every write returns, so an
/// interrupted commit leaves the slice `Built` and a retry re-writes the
/// full set.
pub(super) fn write_three_way_baselines(merged: &[MergePreviewEntry]) -> Result<(), Error> {
    for entry in merged {
        bytes_write(&entry.baseline_path, entry.result.output.as_bytes()).map_err(|err| {
            Error::Diag {
                code: "merge-write-baseline-failed",
                detail: format!(
                    "failed to write baseline {}: {err}",
                    entry.baseline_path.display()
                ),
            }
        })?;
    }
    Ok(())
}

/// For every [`MergeStrategy::OpaqueReplace`] class, recursively copy
/// `class.staged_dir` over `class.baseline_dir`, returning the per-class
/// count of files copied. Empty staged directories are skipped without
/// recording an entry, so the resulting map only carries classes that
/// actually contributed work.
pub(super) fn commit_opaque(classes: &[ArtifactClass]) -> Result<BTreeMap<String, usize>, Error> {
    let mut opaque_counts: BTreeMap<String, usize> = BTreeMap::new();
    for class in classes.iter().filter(|c| matches!(c.strategy, MergeStrategy::OpaqueReplace)) {
        if !class.staged_dir.is_dir() {
            continue;
        }
        let copied = copy_opaque(&class.staged_dir, &class.baseline_dir)?;
        if !copied.is_empty() {
            opaque_counts.insert(class.name.clone(), copied.len());
        }
    }
    Ok(opaque_counts)
}

/// Recursively copy all files from `src` into `dest`, preserving the
/// relative directory structure. Existing files at the same relative
/// path are replaced (opaque whole-file replacement, not delta-merge).
/// Returns the list of relative paths that were copied.
fn copy_opaque(src: &Path, dest: &Path) -> Result<Vec<String>, Error> {
    let mut copied = Vec::new();
    copy_opaque_recursive(src, dest, src, &mut copied)?;
    Ok(copied)
}

fn copy_opaque_recursive(
    base: &Path, dest_base: &Path, current: &Path, copied: &mut Vec<String>,
) -> Result<(), Error> {
    for entry in fs::read_dir(current).map_err(|err| Error::Filesystem {
        op: "readdir",
        path: current.to_path_buf(),
        source: err,
    })? {
        let entry = entry.map_err(|err| Error::Filesystem {
            op: "dir-entry",
            path: current.to_path_buf(),
            source: err,
        })?;
        let path = entry.path();
        let relative = path.strip_prefix(base).map_err(|_err| Error::Filesystem {
            op: "path-prefix",
            path: path.clone(),
            source: std::io::Error::other(format!(
                "path {} is not under base {}",
                path.display(),
                base.display()
            )),
        })?;
        let dest_path = dest_base.join(relative);

        if path.is_dir() {
            fs::create_dir_all(&dest_path).map_err(|err| Error::Filesystem {
                op: "mkdir",
                path: dest_path.clone(),
                source: err,
            })?;
            copy_opaque_recursive(base, dest_base, &path, copied)?;
        } else {
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent).map_err(|err| Error::Filesystem {
                    op: "mkdir",
                    path: parent.to_path_buf(),
                    source: err,
                })?;
            }
            fs::copy(&path, &dest_path).map_err(|err| Error::Filesystem {
                op: "copy",
                path: path.clone(),
                source: err,
            })?;
            copied.push(relative.to_string_lossy().to_string());
        }
    }
    Ok(())
}

/// Build the operator-facing summary stamped onto the merge phase
/// outcome. Format: `Merged <count> <class>[, <count> <class>]* into
/// baseline`. Empty merges (no work) round-trip as
/// `Merged 0 entries into baseline` so the field is never blank.
pub(super) fn build_merge_summary(
    three_way: &[MergePreviewEntry], opaque_counts: &BTreeMap<String, usize>,
) -> String {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for entry in three_way {
        *counts.entry(entry.class_name.clone()).or_insert(0) += 1;
    }
    for (name, count) in opaque_counts {
        *counts.entry(name.clone()).or_insert(0) += count;
    }
    if counts.is_empty() {
        return "Merged 0 entries into baseline".to_string();
    }
    let parts: Vec<String> =
        counts.iter().map(|(class, count)| format!("{count} {class}")).collect();
    format!("Merged {} into baseline", parts.join(", "))
}

#[cfg(test)]
mod tests;
