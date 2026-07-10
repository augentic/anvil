//! The verb-layer failure type and its single HTTP projection.
//!
//! [`Error`] wraps the workspace [`error::Error`] taxonomy and adds
//! the one shape the CLI's two-channel contract needs: a failing
//! validate renders its [`ReportBody`] on stdout *and* the failure
//! envelope on stderr, so the report rides the error to the transport
//! instead of being written from inside the handler.
//!
//! HTTP statuses derive solely from [`Error::status`] — the
//! taxonomy → status projection the plan locks (validation/argument →
//! 422, version floor → 426, everything else → 500). `Exit` stays in
//! `crates/argv`; there is no second table.

use super::output::ReportBody;

/// Failure currency for every verb handler.
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

    /// HTTP status for this failure — the single taxonomy → status
    /// projection: validation and argument failures are 422, a version
    /// floor is 426 (upgrade required), everything else is 500.
    #[must_use]
    pub const fn status(&self) -> u16 {
        match self.core() {
            error::Error::Validation { .. } | error::Error::Argument { .. } => 422,
            error::Error::CliTooOld { .. } | error::Error::AdapterCliTooOld { .. } => 426,
            _ => 500,
        }
    }
}

impl From<Error> for omnia_guest::Error {
    fn from(err: Error) -> Self {
        let core = err.core();
        let mut body = serde_json::json!({
            "error": core.variant_str(),
            "message": core.to_string(),
        });
        if let Error::Report { body: report, .. } = &err
            && let Ok(findings) = serde_json::to_value(report)
        {
            body["report"] = findings;
        }
        Self::Json {
            code: err.status().to_string(),
            body,
        }
    }
}

impl From<Error> for omnia_guest::api::HttpError {
    fn from(err: Error) -> Self {
        omnia_guest::Error::from(err).into()
    }
}
