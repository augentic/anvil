//! Adapter resolution (RFC-64: one component, no manifest).
//!
//! An adapter is a single WebAssembly component. Identity lives in the
//! wasm-pkg package reference (`augentic:<name>@<semver>`), axis in the
//! exported world (`source` xor `target`), and the remaining metadata
//! in the component's own deterministic `describe` answer, dispatched
//! host-side at resolve time and cached against the component digest
//! (see [`describe`]).
//!
//! Source and target adapters split into [`SourceAdapter`] /
//! [`TargetAdapter`] in memory, each carrying its closed operation set
//! ([`SourceOperation`] / [`TargetOperation`]) derived from the closed
//! WIT contract (`wit/specify.wit`). See [DECISIONS.md §"Operations
//! typed at parse boundary"] for the rationale.
//!
//! Resolution keys on the [`AdapterRef`] identity: a pinned
//! `(name, version)` resolves the single-file global store entry at
//! `<store-root>/<name>@<version>.wasm` (RFC-48 D5, verify-on-read
//! included); a bare name resolves the development release build at
//! `target/wasm32-wasip2/release/specify_<name>.wasm` under the project
//! or the sibling `specify-adapters` checkout.
//!
//! [DECISIONS.md §"Operations typed at parse boundary"]: ../../../DECISIONS.md#operations-typed-at-parse-boundary

mod core;
pub mod describe;
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
