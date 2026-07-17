//! Persistent-sandbox lifecycle helpers shared by trial wrappers.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, ensure};
use change::Plan;
use project::config::Layout;

/// Replace any previous project at `root` with an empty directory.
///
/// # Errors
///
/// Returns removal and creation I/O failures.
pub fn replace(root: &Path) -> Result<PathBuf> {
    if root.exists() {
        fs::remove_dir_all(root).context("replacing the previous trial project")?;
    }
    fs::create_dir_all(root).context("creating the trial project root")?;
    root.canonicalize().context("canonical trial project root")
}

/// Require an initialised project at `root`.
///
/// # Errors
///
/// Returns a missing `.specify/project.yaml`.
pub fn require(root: &Path) -> Result<PathBuf> {
    ensure!(
        root.join(".specify/project.yaml").is_file(),
        "project is not initialised; run `cargo make eval init` first"
    );
    root.canonicalize().context("canonical trial project root")
}

/// Load the project's `plan.yaml`.
///
/// # Errors
///
/// Returns a missing or unparseable plan.
pub fn read_plan(root: &Path) -> Result<Plan> {
    Plan::load(&Layout::new(root).plan_path()).context("loading plan.yaml")
}
