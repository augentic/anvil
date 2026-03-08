use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::engine::Engine;
use crate::pipeline::{Pipeline, RepoGroup};
use crate::registry::Registry;
use crate::status::PipelineStatus;

/// Shared context for all commands that operate on an existing change.
///
/// Loads and validates the pipeline, registry, and status in one place,
/// eliminating the repeated preamble across fan-out, apply, sync, archive,
/// and status commands.
pub struct ChangeContext {
    /// Workspace root (directory containing `registry.toml`).
    pub workspace: PathBuf,
    /// Change identifier.
    pub change: String,
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
    pub fn load(workspace: &Path, engine: &dyn Engine, change: &str) -> Result<Self> {
        let change_dir = workspace.join(engine.changes_dir()).join(change);
        let pipeline = Pipeline::load(&change_dir.join("pipeline.toml"))?;
        let registry = Registry::load(&workspace.join("registry.toml"))?;
        pipeline.validate(&registry, &change_dir)?;
        let status_path = change_dir.join("status.toml");
        let status =
            PipelineStatus::load_or_create(&status_path, change, &pipeline, &registry)?;
        Ok(Self {
            workspace: workspace.to_path_buf(),
            change: change.to_string(),
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
