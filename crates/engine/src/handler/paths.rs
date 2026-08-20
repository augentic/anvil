//! [`ExecutionPaths`] — the project root plus artifact [`Locations`].
//! Every path is a fixed constant relative to a named preopen;
//! kernels read the value and never consult the environment.

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
    /// The deployed layout: `.` is the project-root mount and the
    /// cache root is the named cache preopen — identical strings on
    /// wasm32 (preopen table) and native (invocation directory).
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

    /// The per-project derived cache directory.
    #[must_use]
    pub fn cache_dir(&self) -> PathBuf {
        self.locations.cache_dir()
    }
}
