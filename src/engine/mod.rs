pub mod opsx;

use std::path::Path;

use anyhow::Result;

use crate::brief::ChangeBrief;
use crate::pipeline::RepoGroup;

/// Interface between the orchestrator and the spec engine.
/// All tool-specific logic (OPSX, SpecKit, etc.) lives behind this trait.
/// The orchestrator (CLI commands, pipeline, status, git, agent) is engine-agnostic.
pub trait Engine {
    /// Human-readable engine name (e.g., "opsx", "speckit").
    fn name(&self) -> &str;

    // --- Directory conventions ---

    /// Relative path to the specs directory in a target repo (e.g., "openspec/specs").
    fn specs_dir(&self) -> &str;

    /// Relative path to the changes directory in the hub repo (e.g., "openspec/changes").
    fn changes_dir(&self) -> &str;

    /// Relative path to the archive directory in the hub repo.
    fn archive_dir(&self) -> &str;

    /// Absolute path to a specific change directory within the hub workspace.
    fn change_dir(&self, workspace: &Path, change: &str) -> std::path::PathBuf {
        workspace.join(self.changes_dir()).join(change)
    }

    // --- Planning (hub-side) ---

    /// Build the AI prompt for generating planning artefacts.
    fn propose_prompt(&self, change: &str, description: &str, context: &str) -> String;

    /// List of required artefact paths relative to the change directory.
    /// Used to verify that the agent produced everything needed.
    fn required_artifacts(&self) -> Vec<&str>;

    // --- Distribution (hub → target repos) ---

    /// Copy engine-specific artefacts from the hub change directory into a
    /// target repo working directory. The orchestrator handles clone/branch/PR;
    /// this method handles file placement.
    fn distribute(&self, ctx: &DistributeContext) -> Result<()>;

    // --- Execution (target-side) ---

    /// Build the AI command/prompt for implementing a change in a target repo.
    fn apply_command(&self, change: &str, brief: &ChangeBrief) -> String;

    // --- Brief / paths ---

    /// Format a spec file path from a spec name (e.g. "r9k-xml-ingest" -> "r9k-xml-ingest/spec.md").
    fn spec_file_path(&self, spec_name: &str) -> String;

    /// Relative paths for upstream artefacts as placed in the target repo by `distribute()`.
    fn upstream_paths(&self) -> UpstreamPaths;

    // --- Archive ---

    /// Generate the archive directory name for a completed change.
    fn archive_dirname(&self, change: &str) -> String;
}

/// Paths to upstream artefacts within the engine's distribution directory.
pub struct UpstreamPaths {
    pub design: &'static str,
    pub tasks: &'static str,
    pub pipeline: &'static str,
}

/// Context passed to `Engine::distribute()`.
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
