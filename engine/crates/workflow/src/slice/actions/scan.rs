//! `scan_touched` and `write_touched`: classify `<slice_dir>/specs/*`
//! against the baseline tree and persist the result on `metadata.yaml`.

use std::path::Path;

use specify_error::Error;

use crate::slice::{SliceMetadata, SpecKind, TouchedSpec};

/// Scan `<slice_dir>/specs/*` and classify each adapter as
/// `new` or `modified` against `<specs_dir>/<name>/spec.md`.
///
/// Returns entries sorted by adapter name for stable output. The
/// scan is non-destructive — it does not mutate `metadata.yaml`. The
/// caller typically follows up with [`write_touched`].
///
/// # Errors
///
/// Propagates I/O errors from reading `<slice_dir>/specs/` or its entries.
pub fn scan_touched(slice_dir: &Path, specs_dir: &Path) -> Result<Vec<TouchedSpec>, Error> {
    let specs_root = slice_dir.join("specs");
    if !specs_root.is_dir() {
        return Ok(Vec::new());
    }
    let mut entries: Vec<TouchedSpec> = Vec::new();
    for entry in std::fs::read_dir(&specs_root)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let Some(name) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        // Only classify as touched when a spec.md actually exists; an
        // empty subdirectory is noise left over from init/define work
        // in progress.
        if !entry.path().join("spec.md").is_file() {
            continue;
        }
        let baseline = specs_dir.join(&name).join("spec.md");
        let kind = if baseline.is_file() { SpecKind::Modified } else { SpecKind::New };
        entries.push(TouchedSpec { name, kind });
    }
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(entries)
}

/// Overwrite `metadata.yaml`'s `touched_specs` with `entries`.
///
/// Leaves every other field on the struct untouched, including `status`.
///
/// # Errors
///
/// Returns whatever `SliceMetadata::{load, save}` surfaces — typically
/// missing-metadata, parse, or atomic-write failure.
pub fn write_touched(slice_dir: &Path, entries: Vec<TouchedSpec>) -> Result<SliceMetadata, Error> {
    let mut metadata = SliceMetadata::load(slice_dir)?;
    metadata.touched_specs = entries;
    metadata.save(slice_dir)?;
    Ok(metadata)
}
