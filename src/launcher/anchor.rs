//! Project-root anchoring for one invocation.

use std::path::{Path, PathBuf};

/// Resolve the invocation's project root: the nearest ancestor
/// carrying `.emery/project.yaml`. A miss anchors in-place at the
/// invocation directory — pre-init, so `emery init` stays legal and
/// later verbs fail typed in-guest (`not-initialized`). Filesystem
/// probe errors treat the candidate as uninitialised.
#[must_use]
pub fn root(invoked_dir: &Path) -> PathBuf {
    invoked_dir
        .ancestors()
        .find(|candidate| engine::project::Project::path(candidate).try_exists().unwrap_or(false))
        .map_or_else(|| invoked_dir.to_path_buf(), Path::to_path_buf)
}
