//! Unified error types for the `emery` CLI and its domain crates.
//! Every public function returns `Result<T, Error>`; variants are
//! structured so the binary can route them to exit codes and formats.

pub mod error;

pub use error::Error;

/// Workspace-wide `Result` alias bound to [`Error`].
///
/// Lets call sites write `emery_error::Result<T>` (or `Result<T>`
/// after `use emery_error::Result`) without restating the error
/// parameter; supply an explicit `E` to override on the rare path that
/// returns a non-[`Error`] failure.
pub type Result<T, E = Error> = std::result::Result<T, E>;
