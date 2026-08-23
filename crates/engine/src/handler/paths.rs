//! Preopen-relative execution paths.

use std::path::{Component, Path, PathBuf};

use super::locations::Locations;

/// Project root and artifact locations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionPaths {
    /// Project-tree mount.
    project_root: PathBuf,
    locations: Locations,
}

impl ExecutionPaths {
    /// Returns the deployed `.`-rooted layout.
    #[must_use]
    pub fn deployed() -> Self {
        Self {
            project_root: PathBuf::from("."),
            locations: Locations,
        }
    }

    /// Directory the `.` mount is anchored at.
    #[must_use]
    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    /// The carried artifact locations.
    #[must_use]
    pub const fn locations(&self) -> &Locations {
        &self.locations
    }
}

/// Expresses `path` relative to the `.` preopen when it sits inside the
/// project. Host-absolute argv (canonicalized `--sources`, file-relative
/// `path` / local adapter entries) otherwise misses the only mount.
pub fn preopen_relative(path: &Path) -> PathBuf {
    if !path.is_absolute() {
        return path.to_path_buf();
    }
    let mut roots = Vec::new();
    push_root(&mut roots, std::fs::canonicalize(".").ok());
    if let Ok(cwd) = std::env::current_dir() {
        push_root(&mut roots, std::fs::canonicalize(&cwd).ok());
        push_root(&mut roots, Some(cwd));
    }
    let forms = [Some(path.to_path_buf()), std::fs::canonicalize(path).ok()];
    for form in forms.into_iter().flatten() {
        for root in &roots {
            if let Ok(rel) = form.strip_prefix(root) {
                return if rel.as_os_str().is_empty() {
                    PathBuf::from(".")
                } else {
                    rel.to_path_buf()
                };
            }
        }
    }
    path.to_path_buf()
}

// `.` and `/` are guest names, not host prefixes of a canonicalized path.
fn push_root(roots: &mut Vec<PathBuf>, root: Option<PathBuf>) {
    let Some(root) = root else {
        return;
    };
    if !root.is_absolute() || !root.components().any(|c| matches!(c, Component::Normal(_))) {
        return;
    }
    if !roots.contains(&root) {
        roots.push(root);
    }
}
