//! [`ExecutionPaths`] — the provider-carried project root and cache
//! placement.
//!
//! Cache placement is deployment configuration, not process state:
//! rather than mutating `SPECIFY_PROJECT_CACHE` at runtime, a
//! composition root constructs the paths value once and the provider
//! carries it. [`ExecutionPaths::operator`] inherits the process-start
//! cache configuration (the environment lookup happens at cache
//! resolution); [`ExecutionPaths::isolated`] pins an explicit cache
//! parent for sandboxed sessions.

use std::path::{Path, PathBuf};

/// Project root plus optional explicit cache parent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionPaths {
    project_root: PathBuf,
    cache_parent: Option<PathBuf>,
}

impl ExecutionPaths {
    /// Operator paths: anchor at `project_root` and inherit the
    /// process-start cache configuration (`SPECIFY_PROJECT_CACHE`,
    /// XDG/HOME fallbacks) at cache-resolution time.
    #[must_use]
    pub fn operator(project_root: impl Into<PathBuf>) -> Self {
        Self {
            project_root: project_root.into(),
            cache_parent: None,
        }
    }

    /// Isolated paths: anchor at `project_root` with per-project cache
    /// directories created beneath the explicit `cache_parent`.
    #[must_use]
    pub fn isolated(project_root: impl Into<PathBuf>, cache_parent: impl Into<PathBuf>) -> Self {
        Self {
            project_root: project_root.into(),
            cache_parent: Some(cache_parent.into()),
        }
    }

    /// Directory the project-root walk starts from.
    #[must_use]
    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    /// Explicit cache parent, when isolated.
    #[must_use]
    pub fn cache_parent(&self) -> Option<&Path> {
        self.cache_parent.as_deref()
    }

    /// The same cache placement re-anchored at `project_root` — for
    /// call sites that resolve a different project directory (the
    /// discovered `.specify/` root, a workspace slot) under the
    /// provider's cache configuration.
    #[must_use]
    pub fn with_root(&self, project_root: impl Into<PathBuf>) -> Self {
        Self {
            project_root: project_root.into(),
            cache_parent: self.cache_parent.clone(),
        }
    }

    /// The per-project derived cache directory for this value's root.
    #[must_use]
    pub fn cache_dir(&self) -> PathBuf {
        diagnostics::cache::project_cache_dir_under(self.cache_parent(), &self.project_root)
    }
}
