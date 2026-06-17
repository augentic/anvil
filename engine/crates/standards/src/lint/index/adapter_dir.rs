//! Dedicated adapter-directory discovery pass per the standards layer's
//! scoped `adapter_dir` fact family.
//!
//! Walks only the immediate children of `adapters/sources/` and
//! `adapters/targets/` and emits one [`AdapterDir`] fact per child
//! directory, regardless of whether the directory carries an
//! `adapter.yaml` manifest. Symlinked entries are skipped, mirroring the
//! retiring `adapter` WASI tool's `check_missing_manifest`
//! (`file_type.is_symlink()` → skip). Directories are not files, so the
//! facts are appended to the model WITHOUT touching
//! [`crate::lint::WorkspaceModel::files`] — no other rule's candidate set
//! changes.
//!
//! Listing every adapter directory (not just orphans) is deliberate: the
//! `kind: cross-reference` evaluator performs the set-difference against
//! [`crate::lint::AdapterManifest`] itself, so the "missing manifest"
//! join lives in the generic evaluator rather than baked into this
//! extractor. The only path knowledge here is the `adapters/{sources,
//! targets}` axis layout — mechanism, not policy.

use std::path::Path;

use crate::lint::{AdapterAxis, AdapterDir};

/// Discover every immediate adapter directory under `project_dir`,
/// returning the [`AdapterDir`] facts sorted by path.
#[must_use]
pub fn extract(project_dir: &Path) -> Vec<AdapterDir> {
    let mut dirs: Vec<AdapterDir> = Vec::new();
    for (axis_str, axis) in [("sources", AdapterAxis::Sources), ("targets", AdapterAxis::Targets)] {
        let axis_dir = project_dir.join("adapters").join(axis_str);
        collect_axis(project_dir, &axis_dir, axis, &mut dirs);
    }
    dirs.sort_by(|a, b| a.path.cmp(&b.path));
    dirs
}

/// Collect the immediate child directories of one axis directory. A
/// non-existent or unreadable axis directory yields no facts; symlinked
/// and non-directory entries are skipped.
fn collect_axis(project_dir: &Path, axis_dir: &Path, axis: AdapterAxis, out: &mut Vec<AdapterDir>) {
    let Ok(entries) = std::fs::read_dir(axis_dir) else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() || !file_type.is_dir() {
            continue;
        }
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()).map(str::to_owned) else {
            continue;
        };
        out.push(AdapterDir {
            path: relative_display(project_dir, &path),
            axis,
            name,
        });
    }
}

fn relative_display(root: &Path, path: &Path) -> String {
    path.strip_prefix(root).unwrap_or(path).to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests;
