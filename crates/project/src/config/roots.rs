//! In-place vs detached change-home detection. No marker file; no
//! ancestor walk for a detached home.

use std::path::{Path, PathBuf};

use error::Error;

use super::{Layout, ProjectConfig};

/// Resolved product and change roots for one invocation.
///
/// `--change-dir` always selects a detached change home. Otherwise the
/// nearest ancestor carrying `.emery/project.yaml` is in-place, and a
/// miss refuses typed (`change-home-unanchored`) — a detached change
/// home is always an explicit operator selection, never inferred from
/// the working directory (D2).
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
    ///
    /// # Errors
    ///
    /// `change-home-unanchored` when no ancestor carries
    /// `.emery/project.yaml` and no `--change-dir` was given — the
    /// working directory is never silently treated as a detached
    /// change home.
    pub fn resolve(invoked_dir: &Path, change_dir: Option<&Path>) -> Result<Self, Error> {
        if let Some(dir) = change_dir {
            return Ok(Self::detached(invoked_dir, dir));
        }
        ProjectConfig::find_root(invoked_dir).map(|product| Self::InPlace { product }).ok_or_else(
            || {
                Error::validation_failed(
                    "change-home-unanchored",
                    "a change home is an explicit selection",
                    format!(
                        "no `.emery/project.yaml` ancestor above {} and no `--change-dir`",
                        invoked_dir.display()
                    ),
                )
            },
        )
    }

    /// The explicit detached selection: `--change-dir` (relative values
    /// join `invoked_dir`).
    #[must_use]
    pub fn detached(invoked_dir: &Path, change_dir: &Path) -> Self {
        let change = if change_dir.is_absolute() {
            change_dir.to_path_buf()
        } else {
            invoked_dir.join(change_dir)
        };
        Self::Detached { change }
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
