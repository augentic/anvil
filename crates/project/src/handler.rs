//! Shared plumbing for workflow operations.
//!
//! Every `specify` command is an
//! [`omnia_guest::api::operation::Operation`] implementation
//! co-located with the domain module that owns its kernel. This module
//! carries the pieces they share: the [`Anchor`] provider capability,
//! per-invocation `Ctx` (crate-private), [`Render`] output rendering, shared
//! [`ReportBody`] diagnostic envelope, and operation-layer [`Error`].
//!
//! The transports stay out: no clap, no stdout, no exit codes here.
//! `crates/transport` owns the typed command/HTTP routers, command grammar,
//! explicit input conversions, and JSON/text projection.

mod anchor;
mod ctx;
mod error;
mod output;

pub use anchor::Anchor;
pub use ctx::Ctx;
pub use error::Error;
pub use output::{Render, ReportBody, ReportRow};

/// Result alias for operation bodies: any `error::Error` coerces via
/// `From`, and the report-carrying failures construct
/// [`Error::Report`] explicitly.
pub type Result<T, E = Error> = std::result::Result<T, E>;
