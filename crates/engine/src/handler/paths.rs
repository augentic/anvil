//! [`ExecutionPaths`] — product/change roots plus artifact [`Locations`].
//!
//! A composition root constructs the value once; kernels read it and
//! never consult the environment themselves.

use std::path::{Path, PathBuf};

#[cfg(not(target_arch = "wasm32"))]
use super::locations::PROJECT_ROOT_ENV;
use super::locations::{DETACHED_ENV, Locations};
use crate::config::Layout;

/// Product/change roots plus the carried artifact locations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionPaths {
    /// Guest `.` mount: product tree when in-place, change home when
    /// detached.
    project_root: PathBuf,
    change_root: PathBuf,
    detached: bool,
    locations: Locations,
}

impl ExecutionPaths {
    /// In-place constructor: `project_root` is the product tree;
    /// the change home is `<product>/.emery/change/`.
    #[must_use]
    pub fn new(project_root: impl Into<PathBuf>, locations: Locations) -> Self {
        let project_root = project_root.into();
        let change_root = Layout::new(&project_root).change_root();
        Self {
            project_root,
            change_root,
            detached: false,
            locations,
        }
    }

    /// Detached constructor: `change_root` is the change home and the
    /// `.` mount; there is no ambient product root.
    #[must_use]
    pub fn detached(change_root: impl Into<PathBuf>, locations: Locations) -> Self {
        let change_root = change_root.into();
        Self {
            project_root: change_root.clone(),
            change_root,
            detached: true,
            locations,
        }
    }

    /// Operator paths: in-place at `project_root`, capturing
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
        let locations = Locations::from_env();
        if std::env::var_os(DETACHED_ENV).is_some() {
            Self::detached(root, locations)
        } else {
            Self::new(root, locations)
        }
    }

    /// The engine guest's paths: `.` is the mount preopen. Detached
    /// when the launcher exported [`DETACHED_ENV`]; otherwise in-place.
    #[must_use]
    pub fn guest() -> Self {
        if std::env::var_os(DETACHED_ENV).is_some() {
            Self::detached(".", Locations::guest())
        } else {
            Self::new(".", Locations::guest())
        }
    }

    /// Directory the guest `.` mount is anchored at.
    #[must_use]
    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    /// Change home: `<product>/.emery/change/` in-place, or the
    /// operator directory when detached.
    #[must_use]
    pub fn change_root(&self) -> &Path {
        &self.change_root
    }

    /// Whether this anchoring has no ambient product root.
    #[must_use]
    pub const fn is_detached(&self) -> bool {
        self.detached
    }

    /// The carried artifact locations.
    #[must_use]
    pub const fn locations(&self) -> &Locations {
        &self.locations
    }

    /// Assemble from a [`crate::config::Roots`] resolution.
    #[must_use]
    pub fn from_roots(roots: &crate::config::Roots, locations: Locations) -> Self {
        if roots.is_detached() {
            Self::detached(roots.change_root(), locations)
        } else {
            Self::new(roots.mount_root(), locations)
        }
    }

    /// Re-anchor in-place at `project_root`. Adapter `--project-dir`
    /// and `Ctx::load` use this; a host cache parent re-keys.
    #[must_use]
    pub fn with_root(&self, project_root: impl Into<PathBuf>) -> Self {
        Self::new(project_root, self.locations.clone())
    }

    /// Typed view over change-home paths for this anchoring.
    #[must_use]
    pub fn layout(&self) -> Layout<'_> {
        Layout::with_change_root(&self.project_root, &self.change_root)
    }

    /// The per-project derived cache directory for this value's `.`
    /// mount (product when in-place, change home when detached).
    #[must_use]
    pub fn cache_dir(&self) -> PathBuf {
        self.locations.project_cache_dir(&self.project_root)
    }
}
