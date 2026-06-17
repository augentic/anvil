//! Shared filesystem helpers for `specify-standards` integration tests.
//!
//! Lives under `tests/<helper>/mod.rs` per Rust's idiom (the host
//! workspace forbids `mod.rs` everywhere else — see
//! `docs/standards/coding-standards.md` §"Module layout"). Domain
//! scaffolding (rules / hints / tool runners) lives in `eval_support`;
//! this module is the single source for generic fixture-staging helpers.

#![allow(dead_code, reason = "shared test helpers; not every integration binary uses every helper")]

// `copy_dir` comes from the workspace-shared helper file; see
// `tests/common/fs_git.rs` at the repo root.
#[path = "../../../../tests/common/fs_git.rs"]
mod fs_git;
pub use fs_git::copy_dir;
