//! [`RequestContext`] — the one typed per-request context (C5).
//!
//! Assembled once where a project-scoped operation enters, so paths,
//! the loaded project, and the version floor are derived exactly once.

use emery_error::Error;

use super::paths::ExecutionPaths;
use crate::project::Project;

/// One request's resolved context: the deployed paths plus the loaded,
/// floor-checked project.
///
/// Operations read this value instead of re-deriving paths or
/// re-loading `project.yaml`; `emery init` is the one pre-project verb
/// and never constructs it.
#[derive(Debug)]
pub struct RequestContext {
    paths: ExecutionPaths,
    project: Project,
}

impl RequestContext {
    /// Assemble the context over the deployed layout: fix the
    /// preopen-relative paths and load `project.yaml` fail-closed (the
    /// version floor included).
    ///
    /// # Errors
    ///
    /// [`Error::NotInitialized`] when the project is absent, plus the
    /// load and floor failures of [`Project::load`].
    pub fn load() -> Result<Self, Error> {
        let paths = ExecutionPaths::deployed();
        let project = Project::load(paths.project_root())?;
        Ok(Self { paths, project })
    }

    /// The deployed execution paths.
    #[must_use]
    pub const fn paths(&self) -> &ExecutionPaths {
        &self.paths
    }

    /// The loaded, floor-checked project.
    #[must_use]
    pub const fn project(&self) -> &Project {
        &self.project
    }
}
