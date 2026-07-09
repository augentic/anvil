//! Triage predicates over diagnostic slices: `blocking`,
//! `blocking_present`, the severity tally, `count_status`, and
//! `renumber`.

use schema::diagnostics::{
    DiagnosticKind, DiagnosticSummary, FindingStatus, Severity, blocking, blocking_present,
    count_status, renumber,
};

use crate::diagnostics_support::diagnostic;

#[test]
fn blocking_tiers_and_kind() {
    assert!(blocking(&diagnostic("a", Severity::Critical)));
    assert!(blocking(&diagnostic("a", Severity::Important)));
    assert!(!blocking(&diagnostic("a", Severity::Suggestion)));
    assert!(!blocking(&diagnostic("a", Severity::Optional)));

    let mut review = diagnostic("a", Severity::Critical);
    review.kind = DiagnosticKind::Review;
    assert!(!blocking(&review), "a review request never gates");
}

#[test]
fn blocking_respects_status() {
    for demoted in [
        FindingStatus::Ignored,
        FindingStatus::Fixed,
        FindingStatus::Accepted,
        FindingStatus::FalsePositive,
    ] {
        let mut d = diagnostic("a", Severity::Critical);
        d.status = Some(demoted);
        assert!(!blocking(&d), "{demoted:?} must not block");
    }

    let mut open = diagnostic("a", Severity::Critical);
    open.status = Some(FindingStatus::Open);
    assert!(blocking(&open), "explicit open still blocks");
}

#[test]
fn blocking_present_scans() {
    let clean = [diagnostic("a", Severity::Suggestion), diagnostic("b", Severity::Optional)];
    assert!(!blocking_present(&clean));

    let dirty = [diagnostic("a", Severity::Suggestion), diagnostic("b", Severity::Important)];
    assert!(blocking_present(&dirty));
}

#[test]
fn summary_tallies_by_severity() {
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
fn count_status_open_bucket() {
    let mut diags = [
        diagnostic("a", Severity::Critical),
        diagnostic("b", Severity::Critical),
        diagnostic("c", Severity::Critical),
    ];
    diags[1].status = Some(FindingStatus::Open);
    diags[2].status = Some(FindingStatus::Accepted);

    assert_eq!(count_status(&diags, None), 2, "None counts unset + open");
    assert_eq!(count_status(&diags, Some(FindingStatus::Accepted)), 1);
    assert_eq!(count_status(&diags, Some(FindingStatus::Fixed)), 0);
}

#[test]
fn renumber_is_sequential() {
    let mut diags = [diagnostic("zzz", Severity::Critical), diagnostic("yyy", Severity::Important)];
    renumber(&mut diags);
    assert_eq!(diags[0].id, "DIAG-0001");
    assert_eq!(diags[1].id, "DIAG-0002");
}
