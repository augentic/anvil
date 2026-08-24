//! Transport-neutral command plumbing.

mod locations;
mod output;
mod paths;

pub use locations::{ADAPTERS_CONTAINER, Locations};
pub(crate) use output::Render;
pub use paths::ExecutionPaths;
pub(crate) use paths::preopen_path;

/// Operation error type: the workspace taxonomy.
pub type Error = emery_error::Error;

/// Result type for command operations.
pub type Result<T, E = Error> = std::result::Result<T, E>;
