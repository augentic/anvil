//! `specify rules sync` — codex materialization (native residue).
//!
//! `sync` materializes the shared codex packs embedded in this binary
//! into the out-of-tree `<project-cache>/codex/`, pinned to the binary
//! version (RM-07), so consumer projects resolve shared `UNI-*` rules
//! without a co-located framework checkout or a manual `--rules-root`.
//! The read-only `export` handler moved to the shared
//! `specify_dispatch::commands::rules::export` (RFC-65 move 1) and
//! runs on both sides of the seam.

pub use specify_dispatch::commands::rules::cli;
pub mod sync;
