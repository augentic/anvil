//! Typed native-host failures outside the workflow error currency.

/// Failures owned by the native host itself.
///
/// Covers catalog construction, direct reference-host startup, and
/// command-router assembly. Workflow operations keep the shared
/// `error::Error` currency; a lazy listener failure crosses the
/// source/target seam as `project::seam::Error::Internal` with the
/// stable `reference-listener-unavailable` detail prefix instead.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Catalog construction rejected an entry.
    #[error("invalid native catalog: {detail}")]
    Catalog {
        /// What the builder rejected.
        detail: String,
    },
    /// The reference listener could not be started.
    #[error("reference-listener-unavailable: {detail}")]
    Listener {
        /// Why the loopback bind failed.
        detail: String,
    },
    /// The typed command route inventory failed to assemble.
    #[error("command router: {detail}")]
    Router {
        /// The deterministic route or argument conflict.
        detail: String,
    },
}
