//! Transport-neutral command plumbing.

mod context;
mod error;
mod locations;
mod output;
mod paths;

pub use context::RequestContext;
pub use error::{Error, FailureBody};
pub use locations::{ADAPTERS_CONTAINER, Locations, STORE_CONTAINER};
pub use output::{Render, ReportBody, ReportRow};
pub use paths::ExecutionPaths;

/// Result type for command operations.
pub type Result<T, E = Error> = std::result::Result<T, E>;
