//! Guest workspace-routing classification, shared by the plan-author
//! and execute orchestrations.
//!
//! Project-scoped work in a workspace needs a slot sync plus a chdir
//! into `workspace/<project>/`; the guest orchestrations have no
//! counterpart, so they refuse rather than write to the wrong tree
//! (workspace plans run hand-driven: `emery plan next`, then the
//! per-slice breakouts). Classification is centralized here — before any
//! adapter lookup — while each operation maps the refusal to its own
//! error code so operators see the verb they actually ran. The
//! read-only `plan status` projection stays slot-aware instead
//! (`status::project::resolve_work_root`) and never refuses.

use error::Error;
use project::config::{Layout, ProjectConfig};
use project::plan::Plan;

/// How a plan root routes under the guest orchestrations.
pub(super) enum Routing {
    /// Single-project plan root — the guest orchestrations may run.
    SingleProject,
    /// A plan entry is scoped to a workspace project slot (carries the
    /// project name for the refusal detail).
    ProjectScoped(String),
    /// The plan root is a workspace (`workspace: true` in
    /// `project.yaml`).
    WorkspaceRoot,
}

impl Routing {
    /// The refusal detail's subject line; `None` for
    /// [`Routing::SingleProject`].
    pub(super) fn refusal_subject(&self) -> Option<String> {
        match self {
            Self::SingleProject => None,
            Self::ProjectScoped(project) => {
                Some(format!("plan entry scoped to project `{project}`"))
            }
            Self::WorkspaceRoot => {
                Some("the plan root is a workspace (`workspace: true` in project.yaml)".to_string())
            }
        }
    }
}

/// Classify the plan root. `plan` is `None` when no plan exists yet
/// (the author scaffold path — a fresh plan has no entries, so only
/// the `workspace: true` discriminator can apply). A project-scoped
/// entry classifies ahead of the workspace flag so the refusal names
/// the concrete entry.
///
/// # Errors
///
/// Propagates the `project.yaml` load failure.
pub(super) fn classify(layout: Layout<'_>, plan: Option<&Plan>) -> Result<Routing, Error> {
    let config = ProjectConfig::load(layout.project_dir())?;
    if let Some(project) =
        plan.and_then(|plan| plan.entries.iter().find_map(|entry| entry.project.clone()))
    {
        return Ok(Routing::ProjectScoped(project));
    }
    if config.workspace {
        return Ok(Routing::WorkspaceRoot);
    }
    Ok(Routing::SingleProject)
}
