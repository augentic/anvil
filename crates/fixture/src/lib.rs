//! Specify's canonical fixture adapter crate.
//!
//! One SDK-native fixture kernel serving every workflow suite and the
//! examples guest:
//!
//! - [`behaviour`] — the deterministic, model-free behaviour core over
//!   the SDK seam DTOs ([`adapter::seam`]), keyed off the routed
//!   adapter id.
//! - [`ops`] — one compile-checked unit type per catalog identity
//!   (success profiles and typed failure profiles alike), implementing
//!   the per-axis operations traits (`adapter::Source` /
//!   `adapter::Target`) over the core.
//! - [`registry`] (host) — the exhaustive linked catalog and the
//!   [`registry::Fixtures`] binding handed to the shared harness.
//! - [`session`] (host) — throw-away project trees over the harness
//!   default layer, plus the RAII current-directory guard.
//! - [`answers`] (host) — the scripted judgment-answer corpus behind
//!   `omnia-testkit`'s FIFO `Scripted` model double.
//! - `wit` (`wasm32` only) — the combined `adapter`-world export
//!   bindings plus the seam mappings the examples guest shims over.
//!
//! The crate speaks the SDK seam DTOs end to end: only the workflow
//! providers (`harness::convert`) widen values onto engine DTOs, and
//! only the WASM guest maps them onto the WIT records.

pub mod behaviour;
pub mod ops;

#[cfg(target_arch = "wasm32")]
pub mod wit;

#[cfg(not(target_arch = "wasm32"))]
pub mod answers;
#[cfg(not(target_arch = "wasm32"))]
pub mod registry;
#[cfg(not(target_arch = "wasm32"))]
pub mod session;

pub use ops::{
    DOCS, FailBuild, FailExtract, FailGuidance, FailMerge, FailSurvey, Fixture, FixtureCode,
    FixtureDocs, MissingOutput,
};
#[cfg(not(target_arch = "wasm32"))]
pub use registry::{Fixtures, catalog};
#[cfg(not(target_arch = "wasm32"))]
pub use session::{Cwd, Session};
