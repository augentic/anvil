//! Deployment-neutral adapter resolution.
//!
//! Workflow operations resolve adapters through the provider-carried
//! [`Resolver`] capability. The shipped [`resolver::Component`]
//! implementation locates one WebAssembly component, dispatches its
//! deterministic `metadata` export, and caches the answer against the
//! component digest. Other deployments provide the same capability
//! without changing workflow kernels.
//!
//! Resolution keys on the [`AdapterRef`] identity: a pinned
//! `(name, version)` resolves the single-file global store entry at
//! `<store-root>/<name>@<version>.wasm` (verify-on-read
//! included); a bare name resolves the project component cache
//! (`<project-cache>/components/<name>.wasm`) then the in-repo
//! development release build at
//! `target/wasm32-wasip2/release/<name>.wasm` under the project.
//! Resolution never probes outside the project tree, cache, or store.

mod core;
pub mod handlers;
pub mod metadata;
pub(crate) mod operation;
pub mod resolver;

pub(crate) use core::PlatformsSurface;
pub use core::{
    AdapterRef, Axis, BuildInputDeclaration, Origin, PlatformsCapability, ResolvedSource,
    ResolvedTarget, SourceAdapter, TargetAdapter,
};

pub use operation::{SourceOperation, TargetOperation};
pub use resolver::Resolver;
pub(crate) use resolver::component_cache_entry;
