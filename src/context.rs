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
    pub workspace: PathBuf,
    pub change: String,
    pub change_dir: PathBuf,
    pub status_path: PathBuf,
    pub pipeline: Pipeline,
    pub registry: Registry,
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

/// Create a temp directory with a consistent naming scheme.
/// Removes any stale directory from a previous run with the same PID.
pub fn temp_dir(label: &str) -> Result<PathBuf> {
    let dir = std::env::temp_dir().join(format!("alc-{label}-{}", std::process::id()));
    if dir.exists() {
        std::fs::remove_dir_all(&dir)?;
    }
    Ok(dir)
}
