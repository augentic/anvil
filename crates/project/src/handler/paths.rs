//! [`ExecutionPaths`] — the provider-carried project root plus
//! artifact [`Locations`].
//!
//! Layout is deployment configuration, not process state: a
//! composition root constructs the paths value once — capturing any
//! `EMERY_HOME` relocation at that single point — and the provider
//! carries it. Kernels, resolvers, and handlers read the carried value
//! and never consult the environment themselves.

use std::path::{Path, PathBuf};

use super::locations::Locations;

/// Project root plus the carried artifact locations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionPaths {
    project_root: PathBuf,
    locations: Locations,
}

impl ExecutionPaths {
    /// The explicit core constructor: anchor at `project_root` with
    /// the given artifact locations. Sandboxed sessions and tests pass
    /// [`Locations::explicit`]; the launcher passes the one
    /// [`Locations`] its invocation captured.
    #[must_use]
    pub fn new(project_root: impl Into<PathBuf>, locations: Locations) -> Self {
        Self {
            project_root: project_root.into(),
            locations,
        }
    }

    /// Operator paths: anchor at `project_root` and capture the
    /// process environment's layout ([`Locations::from_env`]) once,
    /// here. Composition-root only.
    #[cfg(not(target_arch = "wasm32"))]
    #[must_use]
    pub fn operator(project_root: impl Into<PathBuf>) -> Self {
        Self::new(project_root, Locations::from_env())
    }

    /// The engine guest's paths: the project-root mount preopen at
    /// `.` with the guest's store and cache preopens as the carried
    /// locations ([`Locations::guest`]).
    #[must_use]
    pub fn guest() -> Self {
        Self::new(".", Locations::guest())
    }

    /// Directory the project-root walk starts from.
    #[must_use]
    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    /// The carried artifact locations.
    #[must_use]
    pub const fn locations(&self) -> &Locations {
        &self.locations
    }

    /// The same locations re-anchored at `project_root` — for call
    /// sites that resolve a different project directory (the
    /// discovered `.emery/` root, a workspace slot) under the
    /// provider's layout. A host cache parent derives the new
    /// project's digest-keyed directory; a guest per-project cache
    /// root stays the one mounted preopen.
    #[must_use]
    pub fn with_root(&self, project_root: impl Into<PathBuf>) -> Self {
        Self {
            project_root: project_root.into(),
            locations: self.locations.clone(),
        }
    }

    /// The per-project derived cache directory for this value's root.
    #[must_use]
    pub fn cache_dir(&self) -> PathBuf {
        self.locations.project_cache_dir(&self.project_root)
    }
}
