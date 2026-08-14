//! Retained-sandbox lifecycle helpers shared by the case runner.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result};
use project::config::Layout;
use project::plan::Plan;

/// Holds the sandbox's advisory single-writer lock for one case run;
/// dropping releases it.
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
        fs::remove_dir_all(root).context("replacing the previous case sandbox")?;
    }
    fs::create_dir_all(root).context("creating the case sandbox root")?;
    root.canonicalize().context("canonical case sandbox root")
}

/// Load the project's `plan.yaml`.
///
/// # Errors
///
/// Returns a missing or unparseable plan.
pub fn read_plan(root: &Path) -> Result<Plan> {
    let layout = if root.join(".emery").join("project.yaml").is_file() {
        Layout::new(root)
    } else {
        Layout::detached(root)
    };
    Plan::load(&layout.plan_path()).context("loading plan.yaml")
}
