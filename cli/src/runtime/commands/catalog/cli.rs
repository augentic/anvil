//! Clap derive surface for `specify catalog *`. The umbrella `cli.rs`
//! re-exports [`CatalogAction`].

use std::path::PathBuf;

use clap::{Subcommand, ValueEnum};

/// Component-catalog operations on `.specify/design-system/components.yaml`.
#[derive(Subcommand)]
pub enum CatalogAction {
    /// Cluster repeated structures in the composition baseline and
    /// either report them (`--phase report`) or record the names the
    /// build skill / operator parts hand back (`--phase bind`).
    ///
    /// `report` is read-only: it dispatches the deterministic `vectis
    /// infer` tool and prints the name-free cluster report. `bind`
    /// consumes a skill-authored `{ fingerprint → slug }` bindings file
    /// (`--bindings`), reconciles it against the existing catalog under
    /// the no-overwrite + one-skeleton-per-slug bind guards, and
    /// writes `components.yaml` (or prints the diff under `--dry-run`).
    Infer {
        /// Which phase to run — `report` (read-only) or `bind` (writes
        /// the catalog).
        #[arg(long, value_enum)]
        phase: InferPhase,
        /// Minimum distinct screens a structure must span to cluster
        /// (`report` only; forwarded to the tool, default 2).
        #[arg(long)]
        min_occurrences: Option<u32>,
        /// Path to the skill-authored `{ fingerprint → slug }` bindings
        /// file (`bind` only).
        #[arg(long = "bindings")]
        bindings: Option<PathBuf>,
        /// Print the catalog diff without writing (`bind` only).
        #[arg(long = "dry-run")]
        dry_run: bool,
    },
}

/// Which `specify catalog infer` phase to run.
#[derive(Copy, Clone, Debug, ValueEnum, PartialEq, Eq)]
pub enum InferPhase {
    /// Read-only: emit the deterministic, name-free cluster report.
    Report,
    /// Record skill / operator names against fingerprints and write the
    /// catalog.
    Bind,
}
