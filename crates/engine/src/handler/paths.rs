//! [`ExecutionPaths`] — the project root plus artifact [`Locations`].
//!
//! A composition root constructs the value once; kernels read it and
//! never consult the environment themselves.

use std::path::{Path, PathBuf};

use super::locations::Locations;
#[cfg(not(target_arch = "wasm32"))]
use super::locations::PROJECT_ROOT_ENV;

/// The project root plus the carried artifact locations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionPaths {
    /// Guest `.` mount: the project tree.
    project_root: PathBuf,
    locations: Locations,
}

impl ExecutionPaths {
    /// Anchor at `project_root` with the carried `locations`.
    #[must_use]
    pub fn new(project_root: impl Into<PathBuf>, locations: Locations) -> Self {
        Self {
            project_root: project_root.into(),
            locations,
        }
    }

    /// Operator paths: anchored at `project_root`, capturing
    /// [`Locations::from_env`] once. Composition-root only.
    #[cfg(not(target_arch = "wasm32"))]
    #[must_use]
    pub fn operator(project_root: impl Into<PathBuf>) -> Self {
        Self::new(project_root, Locations::from_env())
    }

    /// Host-backend paths: the launcher-exported [`PROJECT_ROOT_ENV`]
    /// (cwd if unset) plus [`Locations::from_env`]. Composition-root only.
    #[cfg(not(target_arch = "wasm32"))]
    #[must_use]
    pub fn host() -> Self {
        let root = std::env::var_os(PROJECT_ROOT_ENV)
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        Self::new(root, Locations::from_env())
    }

    /// The engine guest's paths: `.` is the mount preopen.
    #[must_use]
    pub fn guest() -> Self {
        Self::new(".", Locations::guest())
    }

    /// Directory the guest `.` mount is anchored at.
    #[must_use]
    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    /// The carried artifact locations.
    #[must_use]
    pub const fn locations(&self) -> &Locations {
        &self.locations
    }

    /// The per-project derived cache directory for this value's `.`
    /// mount.
    #[must_use]
    pub fn cache_dir(&self) -> PathBuf {
        self.locations.project_cache_dir(&self.project_root)
    }
}
