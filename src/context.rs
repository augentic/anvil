use std::path::PathBuf;

use anyhow::Result;

use crate::pipeline::{Pipeline, RepoGroup};
use crate::registry::Registry;
use crate::session::Session;
use crate::status::PipelineStatus;

/// Shared context for all commands that operate on an existing change.
///
/// Loads and validates the pipeline, registry, and status in one place,
/// eliminating the repeated preamble across apply, archive, and status commands.
pub struct ChangeContext {
    /// Path to the change directory (specs, pipeline, status).
    pub change_dir: PathBuf,
    /// Path to `status.toml`.
    pub status_path: PathBuf,
    /// Loaded pipeline for this change.
    pub pipeline: Pipeline,
    /// Service registry.
    pub registry: Registry,
    /// Current pipeline status (mutable).
    pub status: PipelineStatus,
}

impl ChangeContext {
    /// Load pipeline, registry, and status for a change, validating integrity.
    pub fn load(session: &Session, change: &str) -> Result<Self> {
        let change_dir = session.engine.change_dir(&session.workspace, change);
        let pipeline = Pipeline::load(&change_dir.join("pipeline.toml"))?;
        let registry = Registry::load(&session.workspace.join("registry.toml"))?;
        pipeline.validate(&registry, &change_dir)?;
        let status_path = change_dir.join("status.toml");
        let status =
            PipelineStatus::load_or_create(&status_path, change, &pipeline, &registry)?;
        Ok(Self {
            change_dir,
            status_path,
            pipeline,
            registry,
            status,
        })
    }

    /// Persist the current status to disk.
    pub fn save_status(&self) -> Result<()> {
        self.status.save(&self.status_path)
    }

    /// Group pipeline targets by repo.
    pub fn groups(&self) -> Result<Vec<RepoGroup>> {
        self.pipeline.group_by_repo(&self.registry)
    }
}
