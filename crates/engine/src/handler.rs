//! Shared plumbing for command operations: [`Anchor`],
//! [`RequestContext`], [`Render`], [`ReportBody`], and the
//! operation-layer [`Error`]. Transports stay out.

mod anchor;
mod context;
mod error;
mod locations;
mod output;
mod paths;

pub use anchor::Anchor;
pub use context::RequestContext;
pub use error::{Error, FailureBody};
pub use locations::{CachePlacement, GUEST_CACHE_MOUNT, Locations};
pub use output::{Render, ReportBody, ReportRow};
pub use paths::ExecutionPaths;

/// Result alias for operation bodies: any `emery_error::Error` coerces via
/// `From`, and the report-carrying failures construct
/// [`Error::Report`] explicitly.
pub type Result<T, E = Error> = std::result::Result<T, E>;
