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

#[cfg(test)]
mod tests {
    use crate::diagnostic::{DiagnosticReport, DiagnosticSummary, FindingLocation};
    use crate::render::{Format, render};
    use crate::test_support::sample_diagnostic;

    fn report(findings: Vec<crate::diagnostic::Diagnostic>) -> DiagnosticReport {
        DiagnosticReport {
            version: crate::diagnostic::DiagnosticReportVersion,
            summary: DiagnosticSummary::from_diagnostics(&findings),
            findings,
        }
    }

    #[test]
    fn empty_report_renders_no_lines() {
        let out = render(Format::Compact, &report(vec![])).expect("compact never errors");
        assert!(out.is_empty(), "empty report yields no lines, got {out:?}");
    }

    #[test]
    fn renders_tab_separated_fields() {
        let out = render(Format::Compact, &report(vec![sample_diagnostic()])).expect("renders");
        assert_eq!(
            out,
            "important\tUNI-014\tcrates/invoice_export/src/config.rs:18:5\tLiteral deployment URL in generated handler\n"
        );
    }

    #[test]
    fn missing_id_and_location_dashes() {
        let mut finding = sample_diagnostic();
        finding.rule_id = None;
        finding.location = None;
        let out = render(Format::Compact, &report(vec![finding])).expect("renders");
        let fields: Vec<&str> = out.trim_end().split('\t').collect();
        assert_eq!(fields[1], "-", "absent rule id renders as dash");
        assert_eq!(fields[2], "-:-:-", "absent location renders as dashes");
    }

    #[test]
    fn partial_location_fills_dashes() {
        let mut finding = sample_diagnostic();
        finding.location = Some(FindingLocation {
            path: "a/b.rs".into(),
            line: Some(7),
            column: None,
            end_line: None,
            end_column: None,
        });
        let out = render(Format::Compact, &report(vec![finding])).expect("renders");
        assert!(out.contains("a/b.rs:7:-\t"), "column-less location uses dash, got {out:?}");
    }

    /// Tab separation means a comma in the title needs no escaping — it
    /// survives verbatim in the title field.
    #[test]
    fn comma_in_title_is_not_escaped() {
        let mut finding = sample_diagnostic();
        finding.title = "Bundle digest, with comma, exceeds policy".into();
        let out = render(Format::Compact, &report(vec![finding])).expect("renders");
        assert!(
            out.contains("\tBundle digest, with comma, exceeds policy\n"),
            "comma survives unescaped, got {out:?}"
        );
    }

    /// One tab-separated line per finding, terminated by a newline.
    #[test]
    fn one_line_per_finding() {
        let out = render(Format::Compact, &report(vec![sample_diagnostic(), sample_diagnostic()]))
            .expect("renders");
        assert_eq!(out.lines().count(), 2, "one line per finding");
    }
}
