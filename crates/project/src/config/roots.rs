//! In-place vs detached change-home detection. No marker file; no
//! ancestor walk for a detached home.

use std::path::{Path, PathBuf};

use super::{Layout, ProjectConfig};

/// Resolved product and change roots for one invocation.
///
/// `--change-dir` always selects a detached change home. Otherwise the
/// nearest ancestor carrying `.emery/project.yaml` is in-place, and a
/// miss treats `invoked_dir` itself as the detached change root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Roots {
    /// Product checkout; change home is `<product>/.emery/change/`.
    InPlace {
        /// Product (target) tree.
        product: PathBuf,
    },
    /// Operator-selected change directory; no ambient product root.
    Detached {
        /// Change home — `<name>` is the change identity, not a subdirectory.
        change: PathBuf,
    },
}

impl Roots {
    /// Resolve roots from the invocation directory and optional
    /// `--change-dir`. Relative `change_dir` joins `invoked_dir`.
    #[must_use]
    pub fn resolve(invoked_dir: &Path, change_dir: Option<&Path>) -> Self {
        if let Some(dir) = change_dir {
            let change = if dir.is_absolute() { dir.to_path_buf() } else { invoked_dir.join(dir) };
            return Self::Detached { change };
        }
        ProjectConfig::find_root(invoked_dir).map_or_else(
            || Self::Detached {
                change: invoked_dir.to_path_buf(),
            },
            |product| Self::InPlace { product },
        )
    }

    /// Directory mounted as the guest's `.` preopen.
    #[must_use]
    pub fn mount_root(&self) -> &Path {
        match self {
            Self::InPlace { product } => product,
            Self::Detached { change } => change,
        }
    }

    /// Change home: `<product>/.emery/change/` in-place, or the
    /// operator directory when detached.
    #[must_use]
    pub fn change_root(&self) -> PathBuf {
        match self {
            Self::InPlace { product } => Layout::new(product).change_root(),
            Self::Detached { change } => change.clone(),
        }
    }

    /// Product root when in-place.
    #[must_use]
    pub fn product_root(&self) -> Option<&Path> {
        match self {
            Self::InPlace { product } => Some(product),
            Self::Detached { .. } => None,
        }
    }

    /// Whether this resolution has no ambient product root.
    #[must_use]
    pub const fn is_detached(&self) -> bool {
        matches!(self, Self::Detached { .. })
    }

    /// Resolve `--from <definition-root>` against the mount root.
    /// Absolute values pass through; relative values join the mount.
    #[must_use]
    pub fn definition_root(&self, from: &Path) -> PathBuf {
        if from.is_absolute() { from.to_path_buf() } else { self.mount_root().join(from) }
    }
}
