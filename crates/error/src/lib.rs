//! Structured workspace errors.

pub mod error;

pub use error::Error;

/// Workspace-wide `Result` alias bound to [`Error`].
pub type Result<T, E = Error> = std::result::Result<T, E>;
