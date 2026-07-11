//! Clap argument types for `specify plugins {doctor, refresh}`.

use std::path::PathBuf;

use clap::Args;

/// Flags for `specify plugins doctor`.
#[derive(Debug, Args)]
pub struct DoctorArgs {
    /// Project directory used for marketplace discovery.
    #[arg(long, default_value = ".")]
    pub project_dir: PathBuf,
    /// Explicit marketplace file path.
    #[arg(long)]
    pub marketplace: Option<PathBuf>,
}

/// Flags for `specify plugins refresh`.
#[derive(Debug, Args)]
pub struct RefreshArgs {
    /// Project directory used for marketplace discovery.
    #[arg(long, default_value = ".")]
    pub project_dir: PathBuf,
    /// Explicit marketplace file path.
    #[arg(long)]
    pub marketplace: Option<PathBuf>,
    /// Apply the cache deletion.
    #[arg(long)]
    pub yes: bool,
}
