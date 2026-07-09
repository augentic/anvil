//! Terminal formatter.
//!
//! Emits a severity tag, rule id, title, source location, and an
//! indented impact/remediation block per diagnostic plus a tally
//! footer mirroring [`crate::diagnostics::diagnostic::DiagnosticSummary`].
//!
//! ANSI escape codes are hand-written so the crate avoids a colour
//! dependency. The `NO_COLOR` environment variable, when set
//! (regardless of value, per <https://no-color.org>), suppresses the
//! escape codes so tests and pipelines get plain text.

use std::fmt::Write as _;

use super::RenderError;
use crate::diagnostics::diagnostic::{
    Diagnostic, DiagnosticReport, FindingLocation, FindingStatus, Severity,
};

/// Render `report` as colourised terminal output.
///
/// # Errors
///
/// Never errors — the [`Result`] return mirrors the uniform
/// [`super::render`] dispatch signature.
pub fn render(report: &DiagnosticReport) -> Result<String, RenderError> {
    let color = std::env::var_os("NO_COLOR").is_none();
    let mut out = String::new();
    let _ = writeln!(out, "Specify review — {} finding(s)", report.findings.len());
    for finding in &report.findings {
        write_finding(&mut out, finding, color);
    }
    let s = &report.summary;
    let _ = writeln!(
        out,
        "Summary: {} critical, {} important, {} suggestion, {} optional",
        s.critical, s.important, s.suggestion, s.optional
    );
    Ok(out)
}

fn write_finding(out: &mut String, finding: &Diagnostic, color: bool) {
    let tag = paint(finding.severity, severity_tag(finding.severity), color);
    let status = status_tag(finding.status);
    let rule = finding.rule_id.as_deref().map_or(String::new(), |id| format!(" {id}"));
    let location = finding
        .location
        .as_ref()
        .map_or(String::new(), |loc| format!(" ({})", format_location(loc)));
    let _ = writeln!(out, "{tag}{status}{rule} {title}{location}", title = finding.title);
    let _ = writeln!(out, "  impact: {}", finding.impact);
    let _ = writeln!(out, "  remediation: {}", finding.remediation);
}

const fn status_tag(status: Option<FindingStatus>) -> &'static str {
    match status {
        Some(FindingStatus::Ignored) => " [ignored]",
        Some(FindingStatus::FalsePositive) => " [false-positive]",
        Some(FindingStatus::Fixed) => " [fixed]",
        Some(FindingStatus::Accepted) => " [accepted]",
        None | Some(FindingStatus::Open) => "",
    }
}

const fn severity_tag(severity: Severity) -> &'static str {
    match severity {
        Severity::Critical => "[CRITICAL]",
        Severity::Important => "[IMPORTANT]",
        Severity::Suggestion => "[SUGGESTION]",
        Severity::Optional => "[OPTIONAL]",
    }
}

fn paint(severity: Severity, text: &str, color: bool) -> String {
    if !color {
        return text.to_owned();
    }
    let code = match severity {
        Severity::Critical => 31,
        Severity::Important => 33,
        Severity::Suggestion => 36,
        Severity::Optional => 34,
    };
    format!("\x1b[{code}m{text}\x1b[0m")
}

fn format_location(loc: &FindingLocation) -> String {
    match (loc.line, loc.column) {
        (Some(line), Some(col)) => format!("{}:{line}:{col}", loc.path),
        (Some(line), None) => format!("{}:{line}", loc.path),
        _ => loc.path.clone(),
    }
}
