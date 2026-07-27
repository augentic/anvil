//! Clap argument types for `emery archive *`. Each `*Args` type
//! mirrors its command's workflow wire input.

use clap::Args;

/// Arguments for `archive prune`.
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
