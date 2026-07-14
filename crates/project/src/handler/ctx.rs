//! [`Ctx`] — the shared project context every project-scoped verb
//! handler loads once inside `handle`.

use std::path::PathBuf;

use error::Error;
use jiff::Timestamp;

use super::Anchor;
use crate::adapter::{ResolvedTarget, Resolver};
use crate::config::{Layout, ProjectConfig};

/// Shared context for every verb that operates inside an initialised
/// `.specify/` project. Created at the top of each `handle` body via
/// [`Ctx::load`] against the provider's [`Anchor`].
#[derive(Debug)]
pub struct Ctx {
    /// Resolved project root — the nearest ancestor of the anchor
    /// carrying `.specify/project.yaml`.
    pub project_dir: PathBuf,
    /// Loaded `.specify/project.yaml`.
    pub config: ProjectConfig,
}

impl Ctx {
    /// Resolve the project root from the provider's anchor, load
    /// `.specify/project.yaml`, and bundle everything into a `Ctx`.
    ///
    /// # Errors
    ///
    /// Returns `Err(Error)` when no project root is found walking up
    /// from the anchor or the project config fails to load.
    pub fn load(anchor: &impl Anchor) -> Result<Self, Error> {
        let project_dir =
            ProjectConfig::find_root(anchor.project_root()).ok_or(Error::NotInitialized)?;
        let config = ProjectConfig::load(&project_dir)?;
        Ok(Self { project_dir, config })
    }

    /// Resolve this project's target adapter into a
    /// [`ResolvedTarget`] via [`crate::target_policy::project_adapter`].
    ///
    /// # Errors
    ///
    /// Returns `workspace-no-adapter` for adapter-less workspace
    /// projects, and propagates adapter-resolution failures.
    pub fn resolve_target_adapter(
        &self, resolver: &impl Resolver,
    ) -> Result<ResolvedTarget, Error> {
        crate::target_policy::project_adapter(resolver, &self.config, &self.project_dir)
    }

    /// Typed view over `.specify/`-anchored paths. Hand this to
    /// [`crate::config::with_state`] in handlers that mutate
    /// `plan.yaml` / `registry.yaml`.
    #[must_use]
    pub fn layout(&self) -> Layout<'_> {
        Layout::new(&self.project_dir)
    }

    /// Single handler-boundary read of the wall clock. Library crates
    /// never call `Timestamp::now()` (architecture §Time injection); a
    /// handler reads `now` here once and threads it into the workflow
    /// functions that stamp serialised artifacts, so tests pin time by
    /// driving those functions with a fixed `Timestamp`.
    // Deliberately a method (not an associated fn), so a future
    // injected test clock has one named home and handler call sites
    // stay uniform (`ctx.now()`).
    #[must_use]
    pub fn now(&self) -> Timestamp {
        Timestamp::now()
    }
}
