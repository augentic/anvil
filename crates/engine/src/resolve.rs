//! Deployment-neutral source-adapter resolution.
//!
//! Operations resolve source adapters through the provider-carried
//! [`Resolver`] capability, keyed on the typed [`AdapterSelector`].

mod core;
pub mod ensure;
pub mod metadata;
pub mod resolver;
mod routed;
mod selector;

pub use core::{Axis, Origin, ResolvedSource, SourceAdapter};

pub use ensure::ComponentMeta;
pub use resolver::Resolver;
pub use routed::RoutedId;
pub use selector::AdapterSelector;
