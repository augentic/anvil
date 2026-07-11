//! Clap argument types for `specify workspace *`.

use std::path::PathBuf;

use clap::Args;

/// Flags for `specify workspace sync`.
#[derive(Debug, Args)]
pub struct SyncArgs {
    /// Specific projects to sync; omit to sync all registry projects.
    pub projects: Vec<String>,
}

/// Flags for `specify workspace prepare`.
#[derive(Debug, Args)]
pub struct PrepareArgs {
    /// Registry project to prepare.
    pub project: String,
    /// Kebab-case umbrella change name.
    #[arg(long)]
    pub change: String,
    /// Active entry source paths allowed to be dirty during resume.
    #[arg(long = "source", value_name = "PATH")]
    pub sources: Vec<PathBuf>,
    /// Adapter-owned output paths allowed to be dirty during resume.
    #[arg(long = "output", value_name = "PATH")]
    pub outputs: Vec<PathBuf>,
}

/// Flags for `specify workspace push`.
#[derive(Debug, Args)]
pub struct PushArgs {
    /// Specific projects to push; omit to push all dirty clones.
    pub projects: Vec<String>,
    /// Show what would happen without making changes.
    #[arg(long)]
    pub dry_run: bool,
}
