//! Diagnostic triage and report coverage.

use emery_diagnostics::{
    DiagnosticKind, DiagnosticSummary, Severity, has_blocking, is_blocking, renumber,
};

use crate::diagnostics_support::diagnostic;

mod diagnostics_support;

#[test]
fn blocking_tiers_and_kind() {
    assert!(is_blocking(&diagnostic("a", Severity::Critical)));
    assert!(is_blocking(&diagnostic("a", Severity::Important)));
    assert!(!is_blocking(&diagnostic("a", Severity::Suggestion)));
    assert!(!is_blocking(&diagnostic("a", Severity::Optional)));

    let mut review = diagnostic("a", Severity::Critical);
    review.kind = DiagnosticKind::Review;
    assert!(!is_blocking(&review), "a review request never gates");
}

#[test]
fn blocking_scan() {
    let clean = [diagnostic("a", Severity::Suggestion), diagnostic("b", Severity::Optional)];
    assert!(!has_blocking(&clean));

    let dirty = [diagnostic("a", Severity::Suggestion), diagnostic("b", Severity::Important)];
    assert!(has_blocking(&dirty));
}

#[test]
fn summary_by_severity() {
    let diags = [
        diagnostic("a", Severity::Critical),
        diagnostic("b", Severity::Important),
        diagnostic("c", Severity::Important),
        diagnostic("d", Severity::Suggestion),
    ];
    assert_eq!(
        DiagnosticSummary::from_diagnostics(&diags),
        DiagnosticSummary {
            critical: 1,
            important: 2,
            suggestion: 1,
            optional: 0
        }
    );
}

#[test]
fn renumber_sequential() {
    let mut diags = [diagnostic("zzz", Severity::Critical), diagnostic("yyy", Severity::Important)];
    renumber(&mut diags);
    assert_eq!(diags[0].id, "DIAG-0001");
    assert_eq!(diags[1].id, "DIAG-0002");
}
