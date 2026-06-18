//! Tab-separated one-diagnostic-per-line formatter — grep- and
//! PR-bot-friendly.
//!
//! Line shape:
//!
//! ```text
//! <severity>\t<rule-id|->\t<path|->:<line|->:<col|->\t<title>
//! ```
//!
//! Tab over comma so that titles, rule ids, and remediation strings
//! (which often contain commas and rarely contain tabs) do not need
//! escaping.

use std::fmt::Write as _;

use super::RenderError;
use crate::diagnostic::{Diagnostic, DiagnosticReport, Severity};

/// Render `report` as one tab-separated line per diagnostic plus a
/// trailing newline.
///
/// # Errors
///
/// Never errors — the [`Result`] return mirrors the uniform
/// [`super::render`] dispatch signature.
pub fn render(report: &DiagnosticReport) -> Result<String, RenderError> {
    let mut out = String::new();
    for finding in &report.findings {
        write_finding(&mut out, finding);
    }
    Ok(out)
}

fn write_finding(out: &mut String, finding: &Diagnostic) {
    let severity = severity_token(finding.severity);
    let rule_id = finding.rule_id.as_deref().unwrap_or("-");
    let (path, line, col) = finding.location.as_ref().map_or_else(
        || ("-", "-".to_owned(), "-".to_owned()),
        |loc| {
            (
                loc.path.as_str(),
                loc.line.map_or_else(|| "-".to_owned(), |n| n.to_string()),
                loc.column.map_or_else(|| "-".to_owned(), |n| n.to_string()),
            )
        },
    );
    let _ =
        writeln!(out, "{severity}\t{rule_id}\t{path}:{line}:{col}\t{title}", title = finding.title);
}

const fn severity_token(severity: Severity) -> &'static str {
    match severity {
        Severity::Critical => "critical",
        Severity::Important => "important",
        Severity::Suggestion => "suggestion",
        Severity::Optional => "optional",
    }
}
