//! Shared plumbing for workflow operations.
//!
//! Carries [`Anchor`], `Ctx`, [`Render`], [`ReportBody`], and the
//! operation-layer [`Error`]. Transports (clap, stdout, exit codes) stay out.

mod anchor;
mod ctx;
mod error;
mod locations;
mod output;
mod paths;

pub use anchor::Anchor;
pub use ctx::Ctx;
pub use error::{Error, FailureBody};
pub use locations::{
    CHANGE_ROOT_ENV, CachePlacement, DETACHED_ENV, GUEST_CACHE_MOUNT, GUEST_SNAPSHOTS_MOUNT,
    GUEST_STAGING_MOUNT, GUEST_STORE_MOUNT, GUEST_WORKSPACES_MOUNT, Locations, PROJECT_ROOT_ENV,
};
pub use output::{Render, ReportBody, ReportRow};
pub use paths::ExecutionPaths;

/// Result alias for operation bodies: any `error::Error` coerces via
/// `From`, and the report-carrying failures construct
/// [`Error::Report`] explicitly.
pub type Result<T, E = Error> = std::result::Result<T, E>;
