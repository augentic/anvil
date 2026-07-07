//! Consolidated integration binary for `specify-runtime`.
//!
//! One binary per crate: each area file is pulled in as a `#[path]` submodule
//! so the harness links once. The areas compose the RFC-61 walking-skeleton
//! deployment — workflow + echo guests over the omnia runtime — in-process
//! and through the real binary. The shared `common` helper builds and locates
//! the wasm32-wasip2 guest artifacts. See
//! [docs/standards/testing.md](../../../docs/standards/testing.md).

#![cfg(not(target_arch = "wasm32"))]

mod common;

#[path = "composed.rs"]
mod composed;
#[path = "mcp.rs"]
mod mcp;
#[path = "widened.rs"]
mod widened;
#[path = "workflow.rs"]
mod workflow;
