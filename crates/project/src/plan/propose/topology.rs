//! Project-topology resolution: normalise persisted project
//! configuration into the request envelope's `projects[]`.
//!
//! Unlike the pure assembly in [`super::catalog`], every branch here
//! touches the filesystem — the workspace branch reads the committed
//! `.emery/topology.lock`; the regular branch resolves the project's
//! target adapter under `project_dir`.

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
/// Two branches, keyed on the workspace discriminator
/// ([`ProjectConfig::workspace`]):
///
/// - **Workspace** — one [`ProjectRef`] per entry in the committed
///   `.emery/topology.lock`, the projection of each member
///   project's `project.yaml`.
///   `name`, `target`, `description`, `surface[]`, `decisions[]`, and
///   `recent[]` come from the cache. An absent cache fails `topology-cache-missing`
///   before planning can continue.
/// - **Single regular project** — one synthesised [`ProjectRef`]:
///   `name` from `project.yaml.name`, `description` from `project.yaml`,
///   `target` formed by resolving `project.yaml.adapter` through
///   [`crate::adapter::Resolver::resolve_target`], plus the live baseline projection
///   (`surface[]`, `decisions[]`, `recent[]`). A regular project reads its
///   own `project.yaml` live as its single source of truth — no cache.
///
/// Both branches touch the filesystem: the workspace branch reads the lock,
/// the regular branch resolves the target adapter under `project_dir`.
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
/// For each topology project that names a `registry.yaml` entry carrying a
/// non-empty `greenfield_seed.domains[]`, returning advisory
/// `greenfield-seed-shadowed` findings for seeds a baseline supersedes:
///
/// - **Greenfield** (`surface[]` empty *and* no `.emery/specs/`): the
///   seed domains project into `surface[]` as domains with empty
///   `requirements[]`, the greenfield analog of the baseline domain list,
///   so a fresh project still routes leads at plan time.
/// - **Shadowed** (`.emery/specs/` exists): the real surface supersedes
///   the seed, so the seed is ignored and a `greenfield-seed-shadowed`
///   info finding suggests removing the now-stale seed.
///
/// `workspace` selects where each project's `.emery/specs/` lives:
/// the project dir itself for a single regular project, or
/// `workspace/<name>/` for a workspace member.
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
