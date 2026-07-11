//! Clap argument types for `specify archive *`.

use clap::Args;

/// Argv mirror of `archive prune`'s wire input
/// (`workflow::slice::handlers::PruneInput`).
#[derive(Clone, Copy, Debug, Args)]
pub struct PruneArgs {
    /// Keep at most this many most-recent archived slices.
    #[arg(long)]
    pub keep: Option<usize>,
    /// Prune archived slices older than this many days.
    #[arg(long = "older-than")]
    pub older_than: Option<i64>,
    /// Report what would be pruned without removing anything.
    #[arg(long)]
    pub dry_run: bool,
}
