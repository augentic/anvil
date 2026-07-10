//! Clap argument types for `specify adapters {sync}`.

use clap::Args;

/// Flags for `specify adapters sync`.
#[derive(Debug, Clone, Copy, Args)]
pub struct SyncArgs {
    /// Fetch nothing; fail when a pinned adapter is not installed.
    #[arg(long)]
    pub frozen: bool,
}
