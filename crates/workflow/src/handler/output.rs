//! Transport-neutral output rendering.
//!
//! The [`Render`] text-rendering trait and shared [`ReportBody`]
//! diagnostic-report envelope.

use std::io::Write;

use schema::diagnostics::{
    Diagnostic, DiagnosticReport, DiagnosticReportVersion, DiagnosticSummary, has_blocking,
    renumber,
};
use serde::Serialize;

/// Text rendering for a verb body.
///
/// The transport-side twin of the body's `Serialize` impl, colocated
/// with the body type so the response shape stays in a single block of
/// code. The CLI front-end calls it for `--format text`; the HTTP
/// transport serialises the same body as JSON and never calls it.
pub trait Render: Serialize {
    /// Write the human-readable rendering of `self` to `w`.
    ///
    /// # Errors
    ///
    /// Propagates the underlying I/O error.
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()>;
}

/// Per-finding text row renderer for a [`ReportBody`].
pub type ReportRow = fn(&mut dyn Write, &Diagnostic) -> std::io::Result<()>;

/// The neutral [`DiagnosticReport`] envelope shared by `slice
/// validate` and `plan validate`, which differ only in the per-finding
/// row formatter and the empty-report line.
///
/// JSON serialises the wire envelope (`{ version, summary, findings }`)
/// verbatim; text renders a PASS/FAIL banner plus one `row`-formatted
/// line per finding. Ids are assigned sequentially at construction.
/// `empty`, when set, replaces the banner entirely for a finding-free
/// report (e.g. `Plan OK`).
#[derive(Debug, Serialize)]
pub struct ReportBody {
    /// The wire report (`{ version, summary, findings }`).
    #[serde(flatten)]
    report: DiagnosticReport,
    #[serde(skip)]
    row: ReportRow,
    #[serde(skip)]
    empty: Option<&'static str>,
}

impl ReportBody {
    /// Assemble the wire report from raw findings: renumber ids, fold
    /// the summary, and attach the text-rendering hooks.
    #[must_use]
    pub fn new(mut findings: Vec<Diagnostic>, empty: Option<&'static str>, row: ReportRow) -> Self {
        renumber(&mut findings);
        Self {
            report: DiagnosticReport {
                version: DiagnosticReportVersion,
                summary: DiagnosticSummary::from_diagnostics(&findings),
                findings,
            },
            row,
            empty,
        }
    }

    /// The wire report (`{ version, summary, findings }`).
    #[must_use]
    pub const fn report(&self) -> &DiagnosticReport {
        &self.report
    }
}

impl Render for ReportBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        if self.report.findings.is_empty()
            && let Some(line) = self.empty
        {
            return writeln!(w, "{line}");
        }
        let banner = if has_blocking(&self.report.findings) { "FAIL" } else { "PASS" };
        writeln!(w, "{banner}")?;
        for finding in &self.report.findings {
            (self.row)(w, finding)?;
        }
        Ok(())
    }
}
