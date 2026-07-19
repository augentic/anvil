//! Specify's canonical mock adapter crate.
//!
//! One SDK-native mock kernel serving every workflow suite and the
//! examples guest:
//!
//! - [`behaviour`] — the deterministic, model-free behaviour core over
//!   the SDK seam DTOs ([`adapter::seam`]), keyed off the routed
//!   adapter id.
//! - [`ops`] — one compile-checked unit type per catalog identity
//!   (success profiles and typed failure profiles alike), implementing
//!   the per-axis operations traits (`adapter::Source` /
//!   `adapter::Target`) over the core.
//! - [`registry`] (host) — the exhaustive native catalog the workflow
//!   suites and the Specify lab bind into the native host.
//! - [`session`] (host) — throw-away project trees over an offline
//!   [`native::Provider`], plus the RAII current-directory guard.
//! - [`invoke`] (host) — typed operation invocation for suites that
//!   inspect an operation's typed output.
//! - [`answers`] (host) — the scripted judgment-answer corpus behind
//!   `omnia-testkit`'s FIFO `Scripted` model double.
//! - [`model`] (host) — the request-recording `Harness` model double
//!   and `mcp_grants`, pending their move upstream to `omnia-testkit`.
//!
//! The crate speaks the SDK seam DTOs end to end: only the workflow
//! providers (the native host's conversion layer) widen values onto
//! engine DTOs. The example components in `examples/wasm/` wire
//! [`ops::Adapter`] straight into the SDK's `source!` / `target!`
//! export macros — no crate-local WIT bindings.

pub mod behaviour;
pub mod ops;

#[cfg(not(target_arch = "wasm32"))]
pub mod answers;
#[cfg(not(target_arch = "wasm32"))]
pub mod invoke;
#[cfg(not(target_arch = "wasm32"))]
pub mod model;
#[cfg(not(target_arch = "wasm32"))]
pub mod registry;
#[cfg(not(target_arch = "wasm32"))]
pub mod session;

pub use ops::{
    Adapter, Code, DOCS, Docs, FailBuild, FailExtract, FailGuidance, FailMerge, FailSurvey,
    MissingOutput,
};
#[cfg(not(target_arch = "wasm32"))]
pub use registry::catalog;
#[cfg(not(target_arch = "wasm32"))]
pub use session::{Cwd, Session};
