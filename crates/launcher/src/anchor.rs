//! Project-root anchoring for one invocation.

use std::path::{Path, PathBuf};

use project::config::ProjectConfig;
use transport::command::selectors::SeedRequest;

/// The project root the deployment anchors at: an explicit
/// `adapter add --project-dir` when argv carries one (relative values
/// anchor at `invoked_dir`), else the nearest ancestor of
/// `invoked_dir` carrying `.emery/project.yaml`, falling back to
/// `invoked_dir` itself for pre-project commands (`init`,
/// `completions`) — the same walk-then-fallback the engine guest sees
/// through its `.` mount.
pub fn project_root(invoked_dir: &Path, seed: Option<&SeedRequest>) -> PathBuf {
    if let Some(dir) = seed.and_then(|request| request.project_dir.as_ref()) {
        return if dir.is_absolute() { dir.clone() } else { invoked_dir.join(dir) };
    }
    ProjectConfig::find_root(invoked_dir).unwrap_or_else(|| invoked_dir.to_path_buf())
}
