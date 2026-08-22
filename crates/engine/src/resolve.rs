//! Deployment-neutral source-adapter resolution.

mod core;
pub mod ensure;
pub mod metadata;
pub mod resolver;
mod routed;
mod selector;

pub use core::{Axis, Origin, ResolvedSource, SourceAdapter};

pub use routed::RoutedId;
pub use selector::AdapterSelector;
