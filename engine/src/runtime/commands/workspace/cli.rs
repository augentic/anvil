//! Clap derive surface for `specify workspace *`. The umbrella
//! `cli.rs` re-exports `WorkspaceAction`.

use std::path::PathBuf;

use clap::Subcommand;

#[derive(Subcommand)]
pub enum WorkspaceAction {
    /// Create symlinks or git clones under `workspace/<name>/`.
    /// No-op when `registry.yaml` is absent.
    Sync {
        /// Specific project(s) to sync; omit to sync all registry projects.
        #[arg()]
        projects: Vec<String>,
    },
    /// Hidden executor helper: prepare one workspace slot on `specify/<change>`.
    #[command(hide = true)]
    Prepare {
        /// Registry project to prepare.
        project: String,
        /// Kebab-case umbrella change name.
        #[arg(long)]
        change: String,
        /// Active entry source path allowed to be dirty during resume.
        #[arg(long = "source", value_name = "PATH")]
        sources: Vec<PathBuf>,
        /// Adapter-owned output path allowed to be dirty during resume.
        #[arg(long = "output", value_name = "PATH")]
        outputs: Vec<PathBuf>,
    },
    /// Push workspace clones to their remote repositories.
    Push {
        /// Specific project(s) to push; omit to push all dirty clones.
        #[arg()]
        projects: Vec<String>,
        /// Show what would happen without making changes.
        #[arg(long)]
        dry_run: bool,
    },
}
