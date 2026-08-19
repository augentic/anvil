//! [`RequestContext`] — the one typed per-request context (C5).
//!
//! Assembled once where a project-scoped operation enters, so paths,
//! the loaded project, and the version floor are derived exactly once.

use emery_error::Error;

use super::anchor::Anchor;
use super::paths::ExecutionPaths;
use crate::project::Project;

/// One request's resolved context: the provider anchoring plus the
/// loaded, floor-checked project.
///
/// Operations read this value instead of re-deriving paths or
/// re-loading `project.yaml`; `emery init` is the one pre-project verb
/// and never constructs it.
#[derive(Debug)]
pub struct RequestContext<'a> {
    paths: &'a ExecutionPaths,
    project: Project,
}

impl<'a> RequestContext<'a> {
    /// Assemble the context from the provider anchoring: capture the
    /// carried paths and load `project.yaml` fail-closed (the version
    /// floor included).
    ///
    /// # Errors
    ///
    /// [`Error::NotInitialized`] when the project is absent, plus the
    /// load and floor failures of [`Project::load`].
    pub fn load(anchor: &'a impl Anchor) -> Result<Self, Error> {
        let paths = anchor.paths();
        let project = Project::load(paths.project_root())?;
        Ok(Self { paths, project })
    }

    /// The provider-carried execution paths.
    #[must_use]
    pub const fn paths(&self) -> &'a ExecutionPaths {
        self.paths
    }

    /// The loaded, floor-checked project.
    #[must_use]
    pub const fn project(&self) -> &Project {
        &self.project
    }
}
