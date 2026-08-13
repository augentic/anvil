//! `system` command group — the RFC-104 definition loop over a
//! definition home (never a product checkout).

use std::path::PathBuf;

use clap::Args;

/// Arguments for `system survey`.
#[derive(Debug, Args)]
pub struct SurveyArgs {
    /// Definition-home directory (defaults to the current directory).
    /// Deployment-consumed: the launcher mounts it as the
    /// invocation's `.` with no `project.yaml` walk and no mkdir.
    #[arg(long)]
    pub dir: Option<PathBuf>,
}
