//! [`ExecutionPaths`] — the project root plus artifact [`Locations`].
//! The root is a fixed constant relative to the `.` preopen; kernels
//! read the value and never consult the environment.

use std::path::{Path, PathBuf};

use super::locations::Locations;

/// The project root plus the carried artifact locations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionPaths {
    /// The `.` mount: the project tree.
    project_root: PathBuf,
    locations: Locations,
}

impl ExecutionPaths {
    /// The deployed layout: `.` is the project-root mount — the same
    /// string on wasm32 (preopen table) and native (invocation
    /// directory).
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
