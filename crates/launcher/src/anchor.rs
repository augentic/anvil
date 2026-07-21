//! Project-root anchoring for one invocation.

use std::path::{Path, PathBuf};

use project::config::ProjectConfig;
use transport::command::selectors::CommandSelectors;

/// The project root the deployment anchors at: an explicit
/// `adapter add --project-dir` when argv carries one (relative values
/// anchor at `invoked_dir`), else the nearest ancestor of
/// `invoked_dir` carrying `.specify/project.yaml`, falling back to
/// `invoked_dir` itself for pre-project commands (`init`,
/// `completions`) — the same walk-then-fallback the engine guest sees
/// through its `.` mount.
pub fn project_root(invoked_dir: &Path, selectors: &CommandSelectors) -> PathBuf {
    if let Some(dir) = selectors.seed.as_ref().and_then(|seed| seed.project_dir.as_ref()) {
        return if dir.is_absolute() { dir.clone() } else { invoked_dir.join(dir) };
    }
    ProjectConfig::find_root(invoked_dir).unwrap_or_else(|| invoked_dir.to_path_buf())
}
