//! Project-topology resolution: normalise persisted project
//! configuration into the request envelope's `projects[]`. Unlike the
//! pure assembly in [`super::catalog`], every branch touches the filesystem.

use error::{Error, Result};

use super::wire::ProjectRef;
use crate::adapter::{AdapterSelector, Resolver};
use crate::config::ProjectConfig;
use crate::handler::ExecutionPaths;

/// Normalise persisted project configuration into the request's
/// `projects[]` topology: one ref synthesised by reading the project's
/// own `project.yaml` live and resolving its target adapter.
///
/// # Errors
///
/// Requires a resolvable target adapter; resolver failures are
/// preserved.
pub fn resolve_topology(
    resolver: &impl Resolver, config: &ProjectConfig, paths: &ExecutionPaths,
) -> Result<Vec<ProjectRef>> {
    regular_topology(resolver, config, paths).map(|project| vec![project])
}

/// Synthesise the sole [`ProjectRef`] for the project.
fn regular_topology(
    resolver: &impl Resolver, config: &ProjectConfig, paths: &ExecutionPaths,
) -> Result<ProjectRef> {
    let adapter_value = config.adapter.as_deref().ok_or_else(|| {
        Error::validation_failed(
            "plan-propose-project-adapter-missing",
            "a project.yaml declares an adapter",
            "project.yaml omits the `adapter` field",
        )
    })?;
    let adapter = resolver.resolve_target(&AdapterSelector::parse(adapter_value)?, paths)?;
    let target =
        crate::identity::target_ref(&adapter.manifest.name, adapter.manifest.version.as_ref());
    let projection = crate::identity::project_baseline(paths.project_root())?;
    Ok(ProjectRef {
        name: config.name.clone(),
        target,
        description: config.description.clone(),
        surface: projection.surface,
        recent: projection.recent,
        decisions: projection.decisions,
        decisions_more: projection.decisions_more,
        platforms: config.platforms.clone(),
    })
}
