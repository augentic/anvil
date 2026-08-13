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

/// Arguments for `system plan`.
#[derive(Debug, Args)]
pub struct PlanArgs {
    /// Definition-home directory (defaults to the current directory).
    /// Deployment-consumed, as on `system survey`.
    #[arg(long)]
    pub dir: Option<PathBuf>,
}

/// Arguments for `system review`.
#[derive(Debug, Args)]
pub struct ReviewArgs {
    /// The wave to review (`migration.yaml` `waves[].id`).
    pub wave: String,
    /// The exact handoff digest reviewed (the `handoffs/<digest>.yaml`
    /// filename stem, or the full `sha256:…` form).
    #[arg(long)]
    pub handoff: String,
    /// Definition-home directory (defaults to the current directory).
    /// Deployment-consumed, as on `system survey`.
    #[arg(long)]
    pub dir: Option<PathBuf>,
}

/// Arguments for `system status`.
#[derive(Debug, Args)]
pub struct StatusArgs {
    /// Definition-home directory (defaults to the current directory).
    /// Deployment-consumed, as on `system survey`.
    #[arg(long)]
    pub dir: Option<PathBuf>,
}
