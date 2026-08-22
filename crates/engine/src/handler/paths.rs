//! Preopen-relative execution paths.

use std::path::{Path, PathBuf};

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
