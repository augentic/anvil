//! Persistent-sandbox lifecycle helpers shared by trial phases.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, ensure};
use project::config::Layout;
use project::plan::Plan;

/// Holds the sandbox's advisory single-writer lock for one trial
/// phase; dropping releases it.
#[derive(Debug)]
pub struct Guard {
    _file: fs::File,
}

/// Take the exclusive advisory lock guarding the persistent sandbox
/// against a second concurrent eval in the same checkout.
///
/// The lock file lives beside the sandbox (`<sandbox>.lock`) so it
/// survives sandbox replacement and cleanup.
///
/// # Errors
///
/// Returns lock-file I/O failures, and a held lock as "another eval is
/// already running".
pub fn single_writer(sandbox: &Path) -> Result<Guard> {
    let path = lock_path(sandbox);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("creating the sandbox parent")?;
    }
    let file = fs::File::create(&path)
        .with_context(|| format!("creating the sandbox lock {}", path.display()))?;
    file.try_lock().map_err(|err| {
        anyhow::anyhow!(
            "another eval is already running against this sandbox ({}): {err}",
            path.display()
        )
    })?;
    Ok(Guard { _file: file })
}

fn lock_path(sandbox: &Path) -> PathBuf {
    let mut name = sandbox.file_name().unwrap_or_default().to_os_string();
    name.push(".lock");
    sandbox.with_file_name(name)
}

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
