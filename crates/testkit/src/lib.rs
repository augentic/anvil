//! Specify's single test-support crate.
//!
//! Every cross-crate test helper lives here, so the integration suites
//! in `crates/*/tests/` and the examples package stay free of `#[path]`
//! splices and per-suite provider copies:
//!
//! - [`adapter`] — the deterministic fixture adapter core (both WIT
//!   axes) shared by the native provider and the WASM adapter guest.
//! - [`provider`] — the unified capability provider (`Anchor + Model +
//!   Resolver + Hydrator + SourceSeam + TargetSeam`) plus the
//!   operation-invocation helpers.
//! - [`model`] — the recorded harness and scripted model double.
//! - [`answers`] — the scripted judgment-answer corpus.
//! - [`goldens`] — request canonicalization and the golden
//!   compare-or-regenerate flow (`REGENERATE_GOLDENS=1`) that pins the
//!   assembled judgment prompts.
//! - [`cmd`], [`fs`], [`mod@env`], [`plan`] — command mocking, filesystem
//!   and git helpers, env guards, and plan builders.
//!
//! The wasm32 build carries only [`adapter`], for the examples guest.

pub mod adapter;

#[cfg(not(target_arch = "wasm32"))]
pub mod answers;
#[cfg(not(target_arch = "wasm32"))]
pub mod cmd;
#[cfg(not(target_arch = "wasm32"))]
pub mod env;
#[cfg(not(target_arch = "wasm32"))]
pub mod fs;
#[cfg(not(target_arch = "wasm32"))]
pub mod goldens;
#[cfg(not(target_arch = "wasm32"))]
pub mod model;
#[cfg(not(target_arch = "wasm32"))]
pub mod plan;
#[cfg(not(target_arch = "wasm32"))]
pub mod provider;

#[cfg(not(target_arch = "wasm32"))]
pub use provider::{Provider, Scripted, report_rule_ids, resolver, run};
