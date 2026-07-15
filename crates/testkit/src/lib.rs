//! Specify's single test-support crate.
//!
//! Every cross-crate test helper lives here, so the integration suites
//! in `crates/*/tests/` and the examples guest stay free of `#[path]`
//! splices and per-suite provider copies:
//!
//! - [`adapter`] — the deterministic fixture adapter core (both WIT
//!   axes) shared by the native provider and the WASM adapter guest.
//! - `wit` (`wasm32` only) — the `adapter`-world export bindings plus
//!   the seam mappings the examples guest shims over.
//! - [`provider`] — the unified capability provider (`Anchor + Model +
//!   Resolver + Hydrator + Source + Target`) plus the
//!   operation-invocation helpers.
//! - [`answers`] — the scripted judgment-answer corpus behind
//!   `omnia-testkit`'s FIFO `Scripted` model double.
//! - [`cmd`], [`fs`], [`mod@env`], [`plan`] — command mocking, filesystem
//!   and git helpers, env guards, and plan builders.
//!
//! The crate is dual-target: [`adapter`] speaks the engine's seam DTOs
//! and compiles for both native and wasm32 (the examples guest links
//! it); only the host-only test-support modules below stay gated.

pub mod adapter;

#[cfg(target_arch = "wasm32")]
pub mod wit;

#[cfg(not(target_arch = "wasm32"))]
pub mod answers;
#[cfg(not(target_arch = "wasm32"))]
pub mod cmd;
#[cfg(not(target_arch = "wasm32"))]
pub mod env;
#[cfg(not(target_arch = "wasm32"))]
pub mod fs;
#[cfg(not(target_arch = "wasm32"))]
pub mod plan;
#[cfg(not(target_arch = "wasm32"))]
pub mod provider;

#[cfg(not(target_arch = "wasm32"))]
pub use provider::{Provider, Scripted, report_rule_ids, resolver, run};
