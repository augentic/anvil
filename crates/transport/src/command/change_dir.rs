//! Shared `--change-dir` flag for change-scoped verbs.

use std::path::PathBuf;

use clap::Args;

/// Optional detached change-home override.
#[derive(Clone, Debug, Default, Args)]
pub struct ChangeDir {
    /// Detached change home. Optional.
    #[arg(long = "change-dir", value_name = "DIR")]
    pub change_dir: Option<PathBuf>,
}
