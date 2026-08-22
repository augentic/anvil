//! Shared plumbing for command operations: [`RequestContext`],
//! [`Render`], [`ReportBody`], and the operation-layer [`Error`].
//! Transports stay out.

mod context;
mod error;
mod locations;
mod output;
mod paths;

pub use context::RequestContext;
pub use error::{Error, FailureBody};
pub use locations::{ADAPTERS_CONTAINER, CACHE_MOUNT, Locations, STORE_CONTAINER};
pub use output::{Render, ReportBody, ReportRow};
pub use paths::ExecutionPaths;

/// Result alias for operation bodies: any `emery_error::Error` coerces via
/// `From`, and the report-carrying failures construct
/// [`Error::Report`] explicitly.
pub type Result<T, E = Error> = std::result::Result<T, E>;
