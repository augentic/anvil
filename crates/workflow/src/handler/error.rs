//! The operation-layer failure type.
//!
//! [`Error`] wraps the workspace [`error::Error`] taxonomy and adds
//! the one shape the CLI's two-channel contract needs: a failing
//! validate renders its [`ReportBody`] on stdout *and* the failure
//! envelope on stderr, so the report rides the error to the transport
//! instead of being written from inside the operation.

use super::output::ReportBody;

/// Failure currency for every command operation.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The plain workspace failure taxonomy.
    #[error(transparent)]
    Core(#[from] error::Error),

    /// A failure carrying a diagnostic report the transport renders
    /// alongside the failure envelope (the validate verbs' contract:
    /// findings on stdout, the payload-free error on stderr).
    #[error("{source}")]
    Report {
        /// The findings rendered on the success channel.
        body: ReportBody,
        /// The payload-free failure for the error channel.
        source: error::Error,
    },
}

impl Error {
    /// The underlying taxonomy error.
    #[must_use]
    pub const fn core(&self) -> &error::Error {
        match self {
            Self::Core(err) | Self::Report { source: err, .. } => err,
        }
    }

    /// Bundle a diagnostic report with a payload-free
    /// [`error::Error::validation_failed`] failure — the gate verbs'
    /// contract (findings on stdout, the `code`-keyed envelope on
    /// stderr, exit 2).
    #[must_use]
    pub fn validation_report(
        body: ReportBody, code: &'static str, rule: impl Into<String>, detail: impl Into<String>,
    ) -> Self {
        Self::Report {
            body,
            source: error::Error::validation_failed(code, rule, detail),
        }
    }
}
