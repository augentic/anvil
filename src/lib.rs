//! The emery guest: the deployment's only `wasi:cli/run` exporter.
//!
//! Everything — the `workflow`-world WIT bindings, the seam provider,
//! and the transport wiring — lives in the `guest` crate; this cdylib
//! is one macro invocation over it and is embedded into the shipped
//! binary by `build.rs` (`EMERY_WASM`).
#![cfg(target_arch = "wasm32")]

guest::export!();
