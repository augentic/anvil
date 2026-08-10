//! Deployment-neutral adapter resolution.
//!
//! Workflow operations resolve adapters through the provider-carried
//! [`Resolver`] capability, keyed on the typed [`AdapterSelector`].

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
    SourceAdapter, TargetAdapter, WritableArtifactDeclaration, WritableArtifactKind,
};

pub use ensure::ComponentMeta;
pub use operation::{SourceOperation, TargetOperation};
pub use resolver::Resolver;
pub use routed::RoutedId;
pub use selector::{AdapterSelector, FIRST_PARTY_NAMESPACE};
