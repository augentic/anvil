//! Per-claim-kind newtypes for the Evidence document.
//!
//! Each closed `kind:` value from `schemas/evidence.schema.json` may
//! grow its own structured shape over time. This module gathers the
//! per-kind Rust types behind a single module entry point; the
//! generic claim body (`id`, `path`, etc.) lives in the parent
//! [`crate::evidence`] module's shared schema.

pub mod example;

pub use example::ExampleClaim;
