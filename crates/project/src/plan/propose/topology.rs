//! Project-topology resolution: normalise persisted project
//! configuration into the request envelope's `projects[]`. Unlike the
//! pure assembly in [`super::catalog`], every branch touches the filesystem.

use std::path::{Path, PathBuf};

use diagnostics::{Artifact, Diagnostic, DiagnosticKind, DiagnosticSource, Severity};
use error::{Error, Result};

use super::wire::ProjectRef;
use crate::adapter::{AdapterSelector, Resolver};
use crate::config::{Layout, ProjectConfig};
use crate::handler::ExecutionPaths;
use crate::registry::catalog::Registry;
use crate::registry::topology::{Surface, TopologyLock};

/// Normalise persisted project configuration into the request's
/// `projects[]` topology.
///
/// A workspace projects one [`ProjectRef`] per entry in the committed
/// `.emery/topology.lock` (an absent cache fails
/// `topology-cache-missing`); a regular project synthesises one ref by
/// reading its own `project.yaml` live and resolving its target
/// adapter — no cache.
///
/// # Errors
///
/// A workspace requires a committed topology lock; a regular project
/// requires a resolvable target adapter. Resolver failures are preserved.
pub fn resolve_topology(
    resolver: &impl Resolver, config: &ProjectConfig, paths: &ExecutionPaths,
) -> Result<Vec<ProjectRef>> {
    if config.workspace {
        workspace_topology(paths.project_root())
    } else {
        regular_topology(resolver, config, paths).map(|project| vec![project])
    }
}

/// Project greenfield-seed domains into seedless surfaces.
///
/// A greenfield project (empty `surface[]`, no `.emery/specs/`) gets
/// its seed domains projected into `surface[]` so a fresh project
/// still routes leads at plan time; a project with `.emery/specs/` is
/// shadowed — the seed is ignored and an advisory
/// `greenfield-seed-shadowed` finding suggests removing it.
#[must_use]
pub fn apply_greenfield_seed(
    topology: &mut [ProjectRef], registry: &Registry, project_dir: &Path, workspace: bool,
) -> Vec<Diagnostic> {
    let mut findings = Vec::new();
    for project in topology.iter_mut() {
        let Some(entry) = registry.projects.iter().find(|p| p.name == project.name) else {
            continue;
        };
        let Some(seed) = &entry.greenfield_seed else {
            continue;
        };
        if seed.domains.is_empty() {
            continue;
        }
        let has_baseline = project_specs_dir(project_dir, &project.name, workspace).is_dir();
        if has_baseline {
            findings.push(seed_shadowed_finding(&project.name));
        } else if project.surface.is_empty() {
            project.surface = seed
                .domains
                .iter()
                .map(|domain| Surface {
                    domain: domain.clone(),
                    requirements: Vec::new(),
                    more: None,
                })
                .collect();
        }
    }
    findings
}

/// Resolve a topology project's `.emery/specs/` directory: the project
/// dir itself for a regular project, `workspace/<name>/` for a workspace
/// member.
fn project_specs_dir(project_dir: &Path, name: &str, workspace: bool) -> PathBuf {
    let root = if workspace {
        project_dir.join("workspace").join(name)
    } else {
        project_dir.to_path_buf()
    };
    Layout::new(&root).emery_dir().join("specs")
}

/// Build one advisory `greenfield-seed-shadowed` info finding.
fn seed_shadowed_finding(project: &str) -> Diagnostic {
    let message = format!(
        "project '{project}' declares a greenfield_seed but already has a baseline \
         (.emery/specs/ exists); the real surface supersedes the seed — remove it from registry.yaml"
    );
    Diagnostic::finding(
        "greenfield-seed-shadowed".to_string(),
        message.clone(),
        message,
        Severity::Suggestion,
        DiagnosticKind::Review,
        DiagnosticSource::Deterministic,
        Artifact::Plan,
        None,
    )
}

/// Project every committed `.emery/topology.lock` entry into a
/// [`ProjectRef`]. Workspace topology is derived from each member
/// project's `project.yaml`, not from `registry.yaml`.
fn workspace_topology(project_dir: &Path) -> Result<Vec<ProjectRef>> {
    let path = Layout::new(project_dir).topology_lock_path();
    let lock = TopologyLock::load(&path)?.ok_or_else(|| {
        Error::validation_failed(
            "topology-cache-missing",
            "a workspace has a committed .emery/topology.lock",
            "workspace plan-time topology requires a generated .emery/topology.lock",
        )
    })?;
    Ok(lock
        .projects
        .into_iter()
        .map(|project| ProjectRef {
            name: project.name,
            target: project.target,
            description: project.description,
            surface: project.surface,
            recent: project.recent,
            decisions: project.decisions,
            decisions_more: project.decisions_more,
            platforms: project.platforms,
        })
        .collect())
}

/// Synthesise the sole [`ProjectRef`] for a single regular project.
fn regular_topology(
    resolver: &impl Resolver, config: &ProjectConfig, paths: &ExecutionPaths,
) -> Result<ProjectRef> {
    let adapter_value = config.adapter.as_deref().ok_or_else(|| {
        Error::validation_failed(
            "plan-propose-project-adapter-missing",
            "a regular project.yaml declares an adapter",
            "non-workspace project.yaml omits the `adapter` field",
        )
    })?;
    let adapter = resolver.resolve_target(&AdapterSelector::parse(adapter_value)?, paths)?;
    let target = crate::registry::topology::target_ref(
        &adapter.manifest.name,
        adapter.manifest.version.as_ref(),
    );
    let projection = crate::registry::identity::project_baseline(paths.project_root())?;
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
