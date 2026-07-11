use schema::diagnostics::{Artifact, Severity};

use super::*;

// Retained in `src`: `plan_finding` is the `pub(crate)` diagnostic
// constructor behind every plan validation emit — no public boundary
// exposes it directly, and the emitted findings are asserted through
// `Plan::validate` in `crates/workflow/tests/plan_validate.rs`.

/// A2/A13: plan validation findings are built directly on the neutral
/// [`schema::diagnostics::Diagnostic`] currency via `plan_finding`. The
/// stable check code becomes the `rule_id`, the offending entry is
/// carried as `slice`, the artifact is `Plan`, and the fingerprint
/// validates.
#[test]
fn plan_finding_builds_canonical_diagnostic() {
    let diagnostic = plan_finding(
        "plan.cycle",
        Severity::Important,
        "dependency cycle: a -> b -> a",
        Some("checkout".to_string()),
    );

    assert_eq!(diagnostic.rule_id.as_deref(), Some("plan.cycle"));
    assert_eq!(diagnostic.severity, Severity::Important);
    assert_eq!(diagnostic.slice.as_deref(), Some("checkout"));
    assert_eq!(diagnostic.artifact, Artifact::Plan);
    assert_eq!(diagnostic.impact, "dependency cycle: a -> b -> a");
    schema::diagnostics::validate_diagnostic(&diagnostic).expect("plan finding is valid");
    assert!(schema::diagnostics::verify_fingerprint(&diagnostic), "fingerprint covers slice");
}

/// A non-blocking `Suggestion` finding never gates per
/// [`schema::diagnostics::blocking`].
#[test]
fn plan_finding_suggestion_is_non_blocking() {
    let diagnostic = plan_finding(
        "plan.orphan-source",
        Severity::Suggestion,
        "source `docs` is unreferenced",
        None,
    );
    assert_eq!(diagnostic.severity, Severity::Suggestion);
    assert!(diagnostic.slice.is_none());
    assert!(!schema::diagnostics::blocking(&diagnostic));
}
