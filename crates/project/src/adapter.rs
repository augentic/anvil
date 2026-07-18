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
//! reference (`specify:<name>@<semver>`) resolves the single-file
//! global store entry at `<store-root>/<name>@<version>.wasm`
//! (verify-on-read included); a bare name or a persisted local
//! component resolves the project component cache
//! (`<project-cache>/components/<name>.wasm`) then the in-repo
//! development release build at
//! `target/wasm32-wasip2/release/<name>.wasm` under the project.
//! Resolution never probes outside the project tree, cache, or store.
//! Deployment provisioning (package hydration, local-component
//! mirroring, catalog matching) is the [`Resolver::ensure_source`] /
//! [`Resolver::ensure_target`] leg; the component kernels live in
//! [`ensure`].

mod core;
pub mod ensure;
pub mod handlers;
pub mod metadata;
pub(crate) mod operation;
pub mod resolver;
mod selector;

pub(crate) use core::PlatformsSurface;
pub use core::{
    Axis, BuildInputDeclaration, Origin, PlatformsCapability, ResolvedSource, ResolvedTarget,
    SourceAdapter, TargetAdapter,
};

pub use ensure::ComponentMeta;
pub use operation::{SourceOperation, TargetOperation};
pub use resolver::Resolver;
pub use selector::{AdapterSelector, FIRST_PARTY_NAMESPACE};
