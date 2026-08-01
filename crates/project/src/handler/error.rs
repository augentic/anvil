//! The operation-layer failure type.
//!
//! [`Error`] wraps the workspace [`error::Error`] taxonomy and adds
//! the one shape the CLI's two-channel contract needs: a failing
//! gate renders its stdout body ([`FailureBody`]) *and* the failure
//! envelope on stderr, so the report rides the error to the transport
//! instead of being written from inside the operation.

use serde::Serialize;

use super::output::{Render, ReportBody};
use crate::plan::StatusBody;

/// The stdout payload an [`Error::Report`] carries beside the failure
/// envelope — a closed set so the transport renders one of the two
/// known report currencies without a trait object.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum FailureBody {
    /// Diagnostic findings (the validate gates' contract).
    Findings(ReportBody),
    /// The plan-status stop card (`plan execute`'s stop contract —
    /// the same `stop:` / `hint:` / `resume:` projection `emery plan
    /// status` renders).
    Status(Box<StatusBody>),
}

impl Render for FailureBody {
    fn render(&self, w: &mut dyn std::io::Write) -> std::io::Result<()> {
        match self {
            Self::Findings(body) => body.render(w),
            Self::Status(body) => body.render(w),
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

    /// Construct [`Error::Report`] carrying the plan-status stop card:
    /// the canonical `stop:` / `hint:` / `resume:` projection on
    /// stdout, `source` on stderr.
    #[must_use]
    pub fn stopped(status: StatusBody, source: error::Error) -> Self {
        Self::Report {
            body: FailureBody::Status(Box::new(status)),
            source,
        }
    }
}
