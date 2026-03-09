pub mod opsx;

use std::path::Path;

use crate::pipeline::RepoGroup;

/// Paths to upstream artefacts within the engine's distribution directory.
pub struct UpstreamPaths {
    pub design: &'static str,
    pub tasks: &'static str,
    pub pipeline: &'static str,
}

/// Context passed to `OpsxEngine::distribute()`.
pub struct DistributeContext<'a> {
    /// Hub workspace root.
    pub workspace: &'a Path,
    /// Change name.
    pub change: &'a str,
    /// Target repo checkout directory.
    pub repo_dir: &'a Path,
    /// The repo group being distributed to.
    pub group: &'a RepoGroup,
}
