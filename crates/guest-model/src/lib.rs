//! The local `Model` capability for the Specify workflow guest.
//!
//! Engine-side mirror of the adapter guests' `specify-guest-kit::model`
//! — the stand-in for the upstream `omnia-guest` `Model` capability.
//! Wasm-free workflow code takes `P: Model` bounds and issues judgment
//! calls through the trait; on `wasm32` the default method body
//! delegates to the `omnia-wasi-model` bindings, and off `wasm32` tests
//! bind [`MockModel`].
//!
//! `model.rs` / `mock.rs` are kept byte-identical to the canonical copy
//! in `specify-adapters` `crates/guest-kit/src/{model,mock}.rs` — edit
//! there first, mirror here.

mod model;

#[cfg(not(target_arch = "wasm32"))]
mod mock;

#[cfg(not(target_arch = "wasm32"))]
pub use mock::MockModel;
pub use model::{Error, Format, McpGrant, Message, Model, Reply, Request, Role, SchemaFormat};
