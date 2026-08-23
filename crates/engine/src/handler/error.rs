//! Operation-layer failures.

use serde::Serialize;

use super::output::{Render, ReportBody};

/// Report payload carried with an operation failure.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum FailureBody {
    /// Diagnostic findings.
    Findings(ReportBody),
}

impl Render for FailureBody {
    fn render(&self, w: &mut dyn std::io::Write) -> std::io::Result<()> {
        match self {
            Self::Findings(body) => body.render(w),
        }
    }
}

/// Command-operation failure.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Workspace failure.
    #[error(transparent)]
    Core(#[from] emery_error::Error),

    /// A failure with a report body.
    #[error("{source}")]
    Report {
        /// Report body.
        body: FailureBody,
        /// Underlying failure.
        source: emery_error::Error,
    },
}

impl Error {
    /// The underlying taxonomy error.
    #[must_use]
    pub const fn core(&self) -> &emery_error::Error {
        match self {
            Self::Core(err) | Self::Report { source: err, .. } => err,
        }
    }

    /// Constructs a validation failure carrying a diagnostic report.
    #[must_use]
    pub fn report(
        body: ReportBody, code: &'static str, rule: impl Into<String>, detail: impl Into<String>,
    ) -> Self {
        Self::Report {
            body: FailureBody::Findings(body),
            source: emery_error::Error::validation_failed(code, rule, detail),
        }
    }
}
