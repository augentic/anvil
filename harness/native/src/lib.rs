//! The Rust-native execution shim.
//!
//! The wasm guest crosses the `specify:adapter` WIT boundary for its
//! judgment seams; this crate binds the same capabilities natively so
//! the whole workflow — verbs, orchestrators, adapter operations — runs
//! without a wasm runtime:
//!
//! - [`provider`] — [`provider::NativeProvider`]: `workflow::verb::Anchor` +
//!   `omnia_guest::Model` + `SourceSeam` / `TargetSeam` as an
//!   in-process dispatch table over the sibling adapter crates'
//!   `operations` modules, plus the matching describe runner.
//! - [`model`] — the native `Model` backends: [`model::CursorModel`]
//!   (cursor-agent, live-only) and [`model::ReplayModel`] (recorded
//!   fixtures, CI-safe); tests bind `testkit::MockModel` through the
//!   provider's generic parameter.
//! - [`mcp`] — the per-adapter reference shelves, mounted at
//!   `/mcp/<name>` on the serve-mode listener via
//!   `omnia_guest::mcp::router`.
//!
//! What this shim cannot prove: WIT bindings, Omnia's dispatch-by-id,
//! and mount/preopen wiring — that surface stays with the shipped
//! guest (see `harness/README.md`).

pub mod mcp;
pub mod model;
pub mod provider;
