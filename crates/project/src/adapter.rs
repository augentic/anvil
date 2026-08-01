//! Deployment-neutral adapter resolution.
//!
//! Workflow operations resolve adapters through the provider-carried
//! [`Resolver`] capability. The shipped [`resolver::Component`]
//! implementation locates one WebAssembly component, dispatches its
//! deterministic `metadata` export, and caches the answer against the
//! component digest. Other deployments provide the same capability
//! without changing workflow kernels.
//!
//! Resolution keys on the typed [`AdapterSelector`]: a package
//! reference (`emery:<name>@<semver>`) dispatches metadata by
//! routed id — under the shipped deployment the host resolver backs
//! the id with the single-file global store entry at
//! `<store-root>/<name>@<version>.wasm` and installs a miss from the
//! fixed first-party registry (pull-on-miss). A bare name or a
//! persisted local component resolves the seeded project component
//! cache (`<project-cache>/components/<name>.wasm`, populated by
//! `emery adapter add` or a local component at init) when an entry
//! exists; a bare cache miss dispatches the unversioned routed id
//! instead, letting the deployment resolve it local-first (the newest
//! installed store version, with a pull-latest provisioning leg when
//! nothing local exists). Resolution never probes outside the cache
//! or store.
//! Deployment provisioning (local-component mirroring, catalog
//! matching) is the [`Resolver::ensure_source`] /
//! [`Resolver::ensure_target`] leg; the component kernels live in
//! [`ensure`].

mod core;
pub mod ensure;
pub mod handlers;
pub mod metadata;
pub(crate) mod operation;
pub mod resolver;
mod routed;
mod selector;
pub mod upgrade;

pub(crate) use core::PlatformsSurface;
pub use core::{
    Axis, BuildInputDeclaration, Origin, PlatformsCapability, ResolvedSource, ResolvedTarget,
    SourceAdapter, TargetAdapter,
};

pub use ensure::ComponentMeta;
pub use operation::{SourceOperation, TargetOperation};
pub use resolver::Resolver;
pub use routed::RoutedId;
pub use selector::{AdapterSelector, FIRST_PARTY_NAMESPACE};
