//! Deployment-neutral adapter resolution.
//!
//! Workflow operations resolve adapters through the provider-carried
//! [`Resolver`] capability, keyed on the typed [`AdapterSelector`].

pub mod catalog;
mod core;
pub mod ensure;
pub mod handlers;
pub mod metadata;
pub(crate) mod operation;
pub mod resolver;
mod routed;
mod selector;
pub mod upgrade;

pub use core::{
    ArtifactDeclaration, Axis, BuildInputDeclaration, Origin, PlatformsCapability,
    PlatformsSurface, ResolvedSource, ResolvedTarget, SourceAdapter, TargetAdapter,
    WritableArtifactKind,
};

pub use ensure::ComponentMeta;
pub use operation::{SourceOperation, TargetOperation};
pub use resolver::Resolver;
pub use routed::RoutedId;
pub use selector::{AdapterSelector, FIRST_PARTY_NAMESPACE};

/// Host-supplied adapter catalog for detached binding (RFC-88 D6).
pub trait Inventory: Send + Sync {
    /// The catalog this deployment compiled or substituted.
    fn inventory(&self) -> &catalog::Catalog;
}
