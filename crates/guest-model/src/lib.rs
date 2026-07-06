//! The local `Model` capability for the Specify workflow guest.
//!
//! Engine-side mirror of the adapter guests' `specify-guest-kit::model`
//! — the documented stand-in for the `Model` capability RFC-61 adds to
//! `omnia-guest::capabilities`. Wasm-free workflow code takes `P: Model`
//! bounds and issues judgment calls through the trait; on `wasm32` the
//! default method body delegates to the `omnia-wasi-model` bindings, and
//! off `wasm32` tests bind [`MockModel`].
//!
//! `model.rs` / `mock.rs` are kept byte-identical to the canonical copy
//! in `specify-adapters` `crates/guest-kit/src/{model,mock}.rs` — edit
//! there first, mirror here. When the upstream capability lands, the
//! swap is an import change plus a request-construction change: the
//! stand-in's `Request::lend_workspace` bool (resolved against the
//! guest's own `"."` preopen at the wasm call site) will not map 1:1
//! onto the upstream `grants.workspace` descriptor lend.

mod model;

#[cfg(not(target_arch = "wasm32"))]
mod mock;

#[cfg(not(target_arch = "wasm32"))]
pub use mock::MockModel;
pub use model::{Error, Format, McpGrant, Message, Model, Reply, Request, Role, SchemaFormat};
