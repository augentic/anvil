//! Deployment-neutral adapter resolution.
//!
//! Workflow operations resolve adapters through the provider-carried
//! [`Resolver`] capability. The shipped [`resolver::Component`]
//! implementation locates one WebAssembly component, dispatches its
//! deterministic `metadata` export, and caches the answer against the
//! component digest. Other deployments provide the same capability
//! without changing workflow kernels.
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
pub mod handlers;
pub mod metadata;
pub(crate) mod operation;
pub mod resolver;

pub use core::{
    AdapterRef, Axis, BuildInputDeclaration, Origin, PlatformsCapability, PlatformsViolation,
    ResolvedSource, ResolvedTarget, SourceAdapter, TargetAdapter, dev_version,
};
pub(crate) use core::PlatformsSurface;

pub use operation::{SourceOperation, TargetOperation};
pub use resolver::{
    Resolver, component_cache_dir, component_cache_entry, dev_component_filename,
    dev_component_paths,
};
