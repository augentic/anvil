//! Project-scoped request context.

use emery_error::Error;
use omnia_guest::StateStore;

use super::paths::ExecutionPaths;
use crate::project::Project;

/// Deployed paths and a floor-checked project.
#[derive(Debug)]
pub struct RequestContext {
    paths: ExecutionPaths,
    project: Project,
}

impl RequestContext {
    /// Loads the request context from deployed storage.
    ///
    /// # Errors
    ///
    /// Propagates [`Project::load`] failures.
    pub async fn load<S: StateStore>(state: &S) -> Result<Self, Error> {
        let paths = ExecutionPaths::deployed();
        let project = Project::load(state).await?;
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
