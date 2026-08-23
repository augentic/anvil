//! Transport-neutral output rendering.

use std::io::Write;

use emery_diagnostics::{
    Diagnostic, DiagnosticReport, DiagnosticReportVersion, DiagnosticSummary, has_blocking,
    renumber,
};
use serde::Serialize;

/// Human-readable rendering for a serializable command body.
pub trait Render: Serialize {
    /// Writes `self` to `w`.
    ///
    /// # Errors
    ///
    /// Propagates I/O errors.
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()>;
}

/// Text renderer for one report finding.
pub type ReportRow = fn(&mut dyn Write, &Diagnostic) -> std::io::Result<()>;

/// A diagnostic report with transport-neutral text rendering.
#[derive(Debug, Serialize)]
pub struct ReportBody {
    /// Wire report.
    #[serde(flatten)]
    report: DiagnosticReport,
    #[serde(skip)]
    row: ReportRow,
}

impl ReportBody {
    /// Builds a report, assigning ids and computing its summary.
    #[must_use]
    pub fn new(mut findings: Vec<Diagnostic>, row: ReportRow) -> Self {
        renumber(&mut findings);
        Self {
            report: DiagnosticReport {
                version: DiagnosticReportVersion,
                summary: DiagnosticSummary::from_diagnostics(&findings),
                findings,
            },
            row,
        }
    }

    /// Returns the wire report.
    #[must_use]
    pub const fn report(&self) -> &DiagnosticReport {
        &self.report
    }
}

impl Render for ReportBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        let banner = if has_blocking(&self.report.findings) { "FAIL" } else { "PASS" };
        writeln!(w, "{banner}")?;
        for finding in &self.report.findings {
            (self.row)(w, finding)?;
        }
        Ok(())
    }
}
