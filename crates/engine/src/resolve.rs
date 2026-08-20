//! Deployment-neutral source-adapter resolution: operations call
//! [`resolver::Component`] over the deployed metadata dispatch,
//! keyed on the typed [`AdapterSelector`].

mod core;
pub mod ensure;
pub mod metadata;
pub mod resolver;
mod routed;
mod selector;

pub use core::{Axis, Origin, ResolvedSource, SourceAdapter};

pub use ensure::ComponentMeta;
pub use routed::RoutedId;
pub use selector::AdapterSelector;
