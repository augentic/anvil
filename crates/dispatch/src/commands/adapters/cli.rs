//! Clap derive surface for `specify adapters {sync}`. The umbrella
//! `cli.rs` re-exports [`AdaptersAction`].

use clap::Subcommand;

/// Verbs under `specify adapters`.
#[derive(Debug, Clone, Copy, Subcommand)]
pub enum AdaptersAction {
    /// Hydrate every declared pinned adapter identity into the global
    /// store (the explicit hydration trigger).
    ///
    /// Reads `project.yaml` (the `adapter:` pin plus the `adapters:`
    /// prefetch list) and `plan.yaml` source pins when a plan is
    /// present, probes the global store per identity, pulls on miss
    /// through the wasm-pkg transport, verifies each entry's digest
    /// (store sidecar and the committed `.specify/adapters.lock`), and
    /// prints the resolved set with per-identity store paths and
    /// digests. A warm store makes sync a no-op probe. Bare, unpinned
    /// names keep project-local resolution and never hydrate.
    Sync {
        /// Fetch nothing: a store miss aborts with the typed
        /// `adapter-not-installed` error naming the identity instead
        /// of pulling. For offline use and reproducibility-strict CI.
        #[arg(long)]
        frozen: bool,
    },
}
