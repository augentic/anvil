//! The specify guest: the deployment's only `wasi:cli/run` exporter.
//!
//! Everything — the `workflow`-world WIT bindings, the seam provider,
//! and the transport wiring — lives in the `guest` crate; this cdylib
//! is one macro invocation so downstream deployments (the change
//! example in `augentic/specify-adapters`) build the identical guest
//! without vendoring sources.
#![cfg(target_arch = "wasm32")]

guest::export!();
