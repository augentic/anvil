//! Typed native-host failures outside the workflow error currency.

/// Failures owned by the native host itself.
///
/// Covers catalog construction and command-router assembly. Workflow
/// operations keep the shared `error::Error` currency.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Catalog construction rejected an entry.
    #[error("invalid native catalog: {detail}")]
    Catalog {
        /// What the builder rejected.
        detail: String,
    },
    /// The typed command route inventory failed to assemble.
    #[error("command router: {detail}")]
    Router {
        /// The deterministic route or argument conflict.
        detail: String,
    },
}
