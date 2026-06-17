//! Terminal formatter.
//!
//! Emits a severity tag, rule id, title, source location, and an
//! indented impact/remediation block per diagnostic plus a tally
//! footer mirroring [`crate::diagnostic::DiagnosticSummary`].
//!
//! ANSI escape codes are hand-written so the crate avoids a colour
//! dependency. The `NO_COLOR` environment variable, when set
//! (regardless of value, per <https://no-color.org>), suppresses the
//! escape codes so tests and pipelines get plain text.

use std::fmt::Write as _;

use super::RenderError;
use crate::diagnostic::{Diagnostic, DiagnosticReport, FindingLocation, FindingStatus, Severity};

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

#[cfg(test)]
mod tests {
    use crate::diagnostic::{
        DiagnosticReport, DiagnosticSummary, FindingLocation, FindingStatus, Severity,
    };
    use crate::render::{Format, render};
    use crate::test_support::sample_diagnostic;

    fn report(findings: Vec<crate::diagnostic::Diagnostic>) -> DiagnosticReport {
        DiagnosticReport {
            version: crate::diagnostic::DiagnosticReportVersion,
            summary: DiagnosticSummary::from_diagnostics(&findings),
            findings,
        }
    }

    /// Render and strip ANSI escape sequences so assertions match the
    /// uncoloured text regardless of whether `NO_COLOR` is set in the
    /// test environment — avoiding a racy `set_var` across parallel
    /// tests.
    fn render_plain(report: &DiagnosticReport) -> String {
        let raw = render(Format::Pretty, report).expect("pretty never errors");
        strip_ansi(&raw)
    }

    fn strip_ansi(input: &str) -> String {
        let mut out = String::with_capacity(input.len());
        let mut chars = input.chars();
        while let Some(ch) = chars.next() {
            if ch == '\x1b' {
                // Skip the CSI introducer `[` and everything up to and
                // including the terminating `m`.
                for esc in chars.by_ref() {
                    if esc == 'm' {
                        break;
                    }
                }
            } else {
                out.push(ch);
            }
        }
        out
    }

    #[test]
    fn header_and_summary_footer_reflect_counts() {
        let out = render_plain(&report(vec![sample_diagnostic()]));
        assert!(out.starts_with("Specify review — 1 finding(s)\n"));
        assert!(out.contains("Summary: 0 critical, 1 important, 0 suggestion, 0 optional"));
    }

    #[test]
    fn finding_line_fields() {
        let out = render_plain(&report(vec![sample_diagnostic()]));
        assert!(out.contains("[IMPORTANT] UNI-014 Literal deployment URL in generated handler"));
        assert!(out.contains("(crates/invoice_export/src/config.rs:18:5)"));
        assert!(out.contains("  impact: "));
        assert!(out.contains("  remediation: "));
    }

    #[test]
    fn status_tag_renders_for_non_open_findings() {
        let mut finding = sample_diagnostic();
        finding.status = Some(FindingStatus::Ignored);
        let out = render_plain(&report(vec![finding]));
        assert!(out.contains("[IMPORTANT] [ignored]"), "ignored status appears, got {out:?}");
    }

    #[test]
    fn open_status_and_no_status_render_no_tag() {
        let mut open = sample_diagnostic();
        open.status = Some(FindingStatus::Open);
        let out = render_plain(&report(vec![open]));
        assert!(!out.contains("[open]"), "open status is not tagged, got {out:?}");
    }

    #[test]
    fn line_only_location_omits_column() {
        let mut finding = sample_diagnostic();
        finding.severity = Severity::Critical;
        finding.location = Some(FindingLocation {
            path: "x/y.rs".into(),
            line: Some(3),
            column: None,
            end_line: None,
            end_column: None,
        });
        let out = render_plain(&report(vec![finding]));
        assert!(out.contains("(x/y.rs:3)"), "line-only location omits column, got {out:?}");
    }

    /// Each demoted status renders its own bracketed tag — the `ignored`
    /// arm is covered above, so pin the remaining three.
    #[test]
    fn each_non_open_status_tag_renders() {
        for (status, tag) in [
            (FindingStatus::Fixed, "[fixed]"),
            (FindingStatus::Accepted, "[accepted]"),
            (FindingStatus::FalsePositive, "[false-positive]"),
        ] {
            let mut finding = sample_diagnostic();
            finding.status = Some(status);
            let out = render_plain(&report(vec![finding]));
            assert!(out.contains(tag), "expected {tag} for {status:?}, got {out:?}");
        }
    }

    /// A finding with neither location nor rule id renders the bare
    /// title line — no parenthesised location, no rule token.
    #[test]
    fn no_location_no_rule_renders_bare_title() {
        let mut finding = sample_diagnostic();
        finding.location = None;
        finding.rule_id = None;
        let out = render_plain(&report(vec![finding]));
        assert!(!out.contains("config.rs"), "no location rendered, got {out:?}");
        assert!(!out.contains("UNI-014"), "no rule token, got {out:?}");
        assert!(
            out.contains("[IMPORTANT] Literal deployment URL in generated handler"),
            "bare title still present, got {out:?}"
        );
    }

    /// The summary footer is emitted even for an empty report so callers
    /// always get the tally line.
    #[test]
    fn empty_report_prints_header() {
        let out = render_plain(&report(vec![]));
        assert!(out.starts_with("Specify review — 0 finding(s)\n"));
        assert!(out.contains("Summary: 0 critical, 0 important, 0 suggestion, 0 optional"));
    }

    /// With colour enabled each severity wraps its tag in the matching
    /// ANSI escape and resets it; the `color` seam keeps this
    /// deterministic without touching the process `NO_COLOR` env.
    #[test]
    fn paint_wraps_tag_in_ansi() {
        assert_eq!(
            super::paint(Severity::Critical, "[CRITICAL]", true),
            "\x1b[31m[CRITICAL]\x1b[0m"
        );
        assert_eq!(
            super::paint(Severity::Important, "[IMPORTANT]", true),
            "\x1b[33m[IMPORTANT]\x1b[0m"
        );
    }

    /// With colour disabled the tag passes through untouched.
    #[test]
    fn paint_passes_through_when_color_disabled() {
        assert_eq!(super::paint(Severity::Critical, "[CRITICAL]", false), "[CRITICAL]");
    }
}
