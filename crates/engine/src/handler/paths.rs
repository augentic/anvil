//! Preopen-relative execution paths.

use std::path::{Component, Path, PathBuf};

use emery_error::Error;

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

/// Normalizes an operator path inside the `.` project preopen.
///
/// # Errors
///
/// Returns [`Error::Argument`] for an absolute path or a relative path
/// that escapes above the project root.
pub fn preopen_path(path: &Path, argument: &'static str) -> Result<PathBuf, Error> {
    if path.is_absolute() {
        return Err(outside_project(path, argument));
    }
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => normalized.push(part),
            Component::ParentDir if normalized.pop() => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(outside_project(path, argument));
            }
        }
    }
    Ok(if normalized.as_os_str().is_empty() { PathBuf::from(".") } else { normalized })
}

fn outside_project(path: &Path, flag: &'static str) -> Error {
    Error::Argument {
        flag,
        detail: format!(
            "path `{}` must be relative to the project preopen `.` and must not escape it",
            path.display()
        ),
    }
}
