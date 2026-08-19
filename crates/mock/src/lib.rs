//! Emery's canonical mock adapter crate.
//!
//! The wasm-free mock source-adapter core behind `mock-component`
//! (the journey's seam fixture); the crate speaks the SDK seam DTOs.

pub mod behaviour;
pub mod ops;

pub use ops::{Adapter, Code, DOCS, Docs, FailExtract, Intent, MissingExtras};
