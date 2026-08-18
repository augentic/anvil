//! The operation-layer failure type.
//!
//! [`Error`] wraps the workspace taxonomy; a failing gate's stdout body
//! ([`FailureBody`]) rides the error to the transport beside the envelope.

use serde::Serialize;

use super::output::{Render, ReportBody};

/// The stdout payload an [`Error::Report`] carries beside the failure
/// envelope — a closed set so the transport renders the known report
/// currency without a trait object.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum FailureBody {
    /// Diagnostic findings (the validate gates' contract).
    Findings(ReportBody),
}

impl Render for FailureBody {
    fn render(&self, w: &mut dyn std::io::Write) -> std::io::Result<()> {
        match self {
            Self::Findings(body) => body.render(w),
        }
    }
}

/// Failure currency for every command operation.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The plain workspace failure taxonomy.
    #[error(transparent)]
    Core(#[from] error::Error),

    /// A failure carrying a stdout body the transport renders
    /// alongside the failure envelope (findings on stdout, the
    /// payload-free error on stderr).
    #[error("{source}")]
    Report {
        /// The body rendered on the success channel.
        body: FailureBody,
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

    /// Construct [`Error::Report`]: a diagnostic report bundled with a
    /// payload-free [`error::Error::validation_failed`] failure — the
    /// gate verbs' contract (findings on stdout, the `code`-keyed
    /// envelope on stderr, exit 2).
    #[must_use]
    pub fn report(
        body: ReportBody, code: &'static str, rule: impl Into<String>, detail: impl Into<String>,
    ) -> Self {
        Self::Report {
            body: FailureBody::Findings(body),
            source: error::Error::validation_failed(code, rule, detail),
        }
    }
}
