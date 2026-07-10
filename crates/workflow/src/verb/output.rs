//! Transport-neutral output currency.
//!
//! The [`Out`] body wrapper every verb returns, the [`Render`]
//! text-rendering trait the CLI front-end consumes, and the shared
//! [`ReportBody`] diagnostic-report envelope.

use std::fmt;
use std::io::Write;

use schema::diagnostics::{
    Diagnostic, DiagnosticReport, DiagnosticReportVersion, DiagnosticSummary, blocking_present,
    renumber,
};
use serde::{Serialize, Serializer};

/// Uniform `Handler::Output` wrapper.
///
/// A local wrapper lets every verb body — verbs-local structs and
/// re-used `workflow` projections alike — satisfy the HTTP transport's
/// `IntoBody` bound without orphan-rule friction: `Out<T>` is local,
/// so the blanket JSON encoding lives here once.
pub struct Out<T>(pub T);

impl<T> fmt::Debug for Out<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Out(..)")
    }
}

impl<T: Serialize> Serialize for Out<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<T: Serialize + Send + Sync> omnia_guest::api::IntoBody for Out<T> {
    fn into_body(self) -> anyhow::Result<Vec<u8>> {
        Ok(serde_json::to_vec(&self.0)?)
    }
}

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
        let banner = if blocking_present(&self.report.findings) { "FAIL" } else { "PASS" };
        writeln!(w, "{banner}")?;
        for finding in &self.report.findings {
            (self.row)(w, finding)?;
        }
        Ok(())
    }
}
