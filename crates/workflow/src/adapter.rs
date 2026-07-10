//! Adapter resolution: one component, no manifest.
//!
//! An adapter is a single WebAssembly component. Identity lives in the
//! wasm-pkg package reference (`specify:<name>@<semver>`), axis in the
//! exported world (`source` xor `target`), and the remaining metadata
//! in the component's own deterministic `describe` answer, dispatched
//! host-side at resolve time and cached against the component digest
//! (see [`describe`]).
//!
//! Source and target adapters split into [`SourceAdapter`] /
//! [`TargetAdapter`] in memory, each carrying its closed operation set
//! ([`SourceOperation`] / [`TargetOperation`]) derived from the closed
//! WIT contract (`wit/specify.wit`).
//!
//! Resolution keys on the [`AdapterRef`] identity: a pinned
//! `(name, version)` resolves the single-file global store entry at
//! `<store-root>/<name>@<version>.wasm` (verify-on-read
//! included); a bare name resolves the development release build at
//! `target/wasm32-wasip2/release/<name>.wasm` under the project
//! or the sibling `specify-adapters` checkout.

mod core;
pub mod describe;
pub mod handlers;
pub(crate) mod operation;
mod resolve;

pub use core::{
    AdapterLocation, AdapterRef, Axis, BuildInputDeclaration, PlatformsCapability,
    PlatformsViolation, ResolvedSourceAdapter, ResolvedTargetAdapter, SourceAdapter, TargetAdapter,
    dev_version,
};

pub use operation::{SourceOperation, TargetOperation};
pub use resolve::{
    component_cache_dir, component_cache_entry, dev_component_filename, dev_component_paths,
};
