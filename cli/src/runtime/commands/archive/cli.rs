//! Clap derive surface for `specify archive *`.

use clap::Subcommand;

#[derive(Subcommand)]
pub enum ArchiveAction {
    /// Prune archived slice folders under `.specify/archive/` that fall
    /// outside the supplied retention bounds.
    ///
    /// The archive is a prunable convenience cache, not the system of
    /// record — git history of `.specify/specs/` plus the
    /// `slice.archive.created` journal entries are. At least one of
    /// `--keep` / `--older-than` is required; a folder is pruned when it
    /// falls outside the newest-`--keep` window or is older than
    /// `--older-than` days.
    Prune {
        /// Keep at most this many most-recent archived slices.
        #[arg(long)]
        keep: Option<usize>,
        /// Prune archived slices older than this many days.
        #[arg(long = "older-than")]
        older_than: Option<i64>,
        /// Report what would be pruned without removing anything.
        #[arg(long)]
        dry_run: bool,
    },
}
