//! Shared plumbing for the command handlers.
//!
//! Every `specify` command is an [`omnia_guest::api::Handler`] impl
//! co-located with the domain module that owns its kernel (each
//! domain's `handlers` submodule). This module carries the pieces all
//! of them share: the [`Anchor`] provider capability, the
//! per-invocation [`Ctx`], the [`Out`] / [`Render`] output currency,
//! the shared [`ReportBody`] diagnostic envelope, and the
//! handler-layer [`Error`] with its single HTTP status projection.
//!
//! The transports stay out: no clap, no stdout, no exit codes here.
//! `crates/cli` owns the argv grammar and the `Reply` → JSON/text
//! rendering; each shim owns its own dispatch match and HTTP route
//! table.

mod anchor;
mod ctx;
mod error;
mod output;

pub use anchor::Anchor;
pub use ctx::Ctx;
pub use error::Error;
pub use output::{Out, Render, ReportBody, ReportRow};

/// Result alias for handler bodies: any `error::Error` coerces via
/// `From`, and the report-carrying failures construct
/// [`Error::Report`] explicitly.
pub type Result<T, E = Error> = std::result::Result<T, E>;
