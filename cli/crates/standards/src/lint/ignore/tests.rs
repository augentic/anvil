use specify_diagnostics::{
    Artifact, Confidence, DiagnosticKind, DiagnosticReport, DiagnosticReportVersion,
    DiagnosticSource, DiagnosticSummary, FindingEvidence, FindingLocation, Severity,
    fingerprint as compute_fingerprint,
};
use specify_error::Error;

use super::*;
use crate::rules::{Origin, PathRoot};

fn directive(
    path: &str, line: u32, rule_id: &str, target_line: u32, rationale: Option<&str>,
) -> IgnoreDirective {
    let raw = rationale.map_or_else(
        || format!("// specify-ignore: {rule_id}"),
        |r| format!("// specify-ignore: {rule_id} — {r}"),
    );
    IgnoreDirective {
        path: path.into(),
        line,
        rule_id: rule_id.into(),
        rationale: rationale.map(str::to_string),
        target_line,
        raw,
    }
}

fn finding(rule_id: &str, path: &str, line: u32) -> Diagnostic {
    let mut f = Diagnostic {
        id: "FIND-0001".into(),
        rule_id: Some(rule_id.into()),
        related_rule_ids: None,
        title: "demo".into(),
        severity: Severity::Important,
        source: DiagnosticSource::Deterministic,
        kind: DiagnosticKind::Violation,
        target_adapter: None,
        source_adapter: None,
        slice: None,
        change: None,
        artifact: Artifact::Code,
        location: Some(FindingLocation {
            path: path.into(),
            line: Some(line),
            column: None,
            end_line: None,
            end_column: None,
        }),
        evidence: FindingEvidence::Snippet {
            value: format!("payload for {rule_id}"),
        },
        impact: "i".into(),
        remediation: "r".into(),
        confidence: Some(Confidence::High),
        fingerprint: String::new(),
        status: None,
        disposition: None,
    };
    f.fingerprint = compute_fingerprint(&f);
    f
}

fn rule(rule_id: &str, severity: Severity) -> ResolvedRule {
    ResolvedRule {
        rule_id: rule_id.into(),
        title: format!("{rule_id} title"),
        severity,
        trigger: format!("Trigger for {rule_id}"),
        lint_mode: None,
        applicability: None,
        rule_hints: None,
        references: None,
        origin: Origin::Shared,
        path_root: PathRoot::RulesRoot,
        path: format!("shared/{rule_id}.md"),
        body: String::new(),
        deprecated: None,
    }
}

fn validation_rules() -> Vec<ResolvedRule> {
    vec![rule(UNI_022, Severity::Important), rule(UNI_023, Severity::Important)]
}

/// Test 1: a directive whose `(path, target_line, rule_id)`
/// matches a finding flips it to `Ignored` and populates the
/// directive disposition verbatim.
#[test]
fn match_demotes_finding_to_ignored() {
    let mut findings = vec![finding("UNI-014", "src/lib.rs", 18)];
    let dirs = vec![directive(
        "src/lib.rs",
        17,
        "UNI-014",
        18,
        Some("legitimate operator-acknowledged exception"),
    )];
    let outcome = apply(&mut findings, &dirs, &validation_rules(), 100);
    assert!(outcome.synthetics.is_empty(), "rationale is long enough; no synthetic expected");
    assert_eq!(outcome.next_id_counter, 100, "no synthetic minted; counter untouched");

    let stamped = &findings[0];
    assert_eq!(stamped.status, Some(FindingStatus::Ignored));
    let disp = stamped.disposition.as_ref().expect("disposition stamped");
    assert_eq!(disp.source, DispositionSource::Directive);
    let inner = disp.directive.as_ref().expect("directive sub-field");
    assert_eq!(inner.path, "src/lib.rs");
    assert_eq!(inner.line, 17);
    assert_eq!(inner.rationale, "legitimate operator-acknowledged exception");
    assert!(disp.since.is_none());
}

/// Test 2: rationale beginning with `false-positive:` demotes to
/// `FalsePositive` instead of `Ignored`; the disposition still
/// carries the verbatim rationale.
#[test]
fn false_positive_prefix_promotes_status() {
    let mut findings = vec![finding("UNI-014", "src/lib.rs", 18)];
    let dirs = vec![directive(
        "src/lib.rs",
        17,
        "UNI-014",
        18,
        Some("false-positive: scanner misfires for the demo stub URL"),
    )];
    let outcome = apply(&mut findings, &dirs, &validation_rules(), 100);
    assert!(outcome.synthetics.is_empty());

    let stamped = &findings[0];
    assert_eq!(stamped.status, Some(FindingStatus::FalsePositive));
    let rationale = &stamped.disposition.as_ref().unwrap().directive.as_ref().unwrap().rationale;
    assert!(rationale.starts_with("false-positive:"));
}

/// Test 3a: directive with `rationale = None` mints UNI-022 when
/// the rule is resolved.
#[test]
fn mints_uni_022_when_resolved() {
    let mut findings = vec![finding("UNI-014", "src/lib.rs", 18)];
    let dirs = vec![directive("src/lib.rs", 17, "UNI-014", 18, None)];
    let outcome = apply(&mut findings, &dirs, &validation_rules(), 100);

    assert_eq!(
        outcome.synthetics.len(),
        1,
        "exactly one UNI-022 expected: {:?}",
        outcome.synthetics
    );
    let synth = &outcome.synthetics[0];
    assert_eq!(synth.rule_id.as_deref(), Some(UNI_022));
    assert_eq!(synth.status, Some(FindingStatus::Open));
    let location = synth.location.as_ref().expect("synthetic carries location");
    assert_eq!(location.path, "src/lib.rs");
    assert_eq!(location.line, Some(17));
    assert!(synth.disposition.is_none());
    assert_eq!(synth.id, "FIND-0100");
    assert_eq!(outcome.next_id_counter, 101);
}

/// Test 3b: missing rationale is silent when UNI-022 is not in
/// the resolved set (graceful degradation when the universal codex
/// tree is absent). The match-and-demote step still runs against
/// the matched finding.
#[test]
fn silent_when_uni_022_absent() {
    let mut findings = vec![finding("UNI-014", "src/lib.rs", 18)];
    let dirs = vec![directive("src/lib.rs", 17, "UNI-014", 18, None)];
    // Only UNI-023 is resolved; UNI-022 absent.
    let resolved = vec![rule(UNI_023, Severity::Important)];
    let outcome = apply(&mut findings, &dirs, &resolved, 100);
    assert!(
        outcome.synthetics.is_empty(),
        "no UNI-022 must be minted; UNI-023 would only fire on an orphan",
    );
    // The match still stamps the finding.
    assert_eq!(findings[0].status, Some(FindingStatus::Ignored));
}

/// Test 4: rationale shorter than 16 characters mints UNI-022
/// with the same gating as test 3.
#[test]
fn short_rationale_mints_uni_022() {
    let mut findings = vec![finding("UNI-014", "src/lib.rs", 18)];
    let dirs = vec![directive("src/lib.rs", 17, "UNI-014", 18, Some("too short"))];
    let outcome = apply(&mut findings, &dirs, &validation_rules(), 1);
    assert_eq!(outcome.synthetics.len(), 1);
    assert_eq!(outcome.synthetics[0].rule_id.as_deref(), Some(UNI_022));

    // Empty codex: no synthetic.
    let mut findings = vec![finding("UNI-014", "src/lib.rs", 18)];
    let dirs = vec![directive("src/lib.rs", 17, "UNI-014", 18, Some("too short"))];
    let outcome = apply(&mut findings, &dirs, &[], 1);
    assert!(outcome.synthetics.is_empty());
}

/// Test 5: directive whose `rule_id` does not match any finding on
/// its target line mints UNI-023 when resolved, and silently when
/// not.
#[test]
fn orphan_directive_mints_uni_023() {
    let mut findings = vec![finding("UNI-014", "src/lib.rs", 18)];
    let dirs = vec![directive(
        "src/lib.rs",
        17,
        "UNI-099",
        18,
        Some("explicit unmatched rule id reference"),
    )];
    let outcome = apply(&mut findings, &dirs, &validation_rules(), 100);
    let ids: Vec<&str> = outcome.synthetics.iter().filter_map(|f| f.rule_id.as_deref()).collect();
    assert!(ids.contains(&UNI_023), "expected UNI-023 in synthetics; got {ids:?}");
    assert!(!ids.contains(&UNI_022), "UNI-022 must not fire for long rationale");
    // The matched finding stays Open.
    assert!(findings[0].status.is_none());

    // Graceful degradation: UNI-023 missing → no synthetic.
    let mut findings = vec![finding("UNI-014", "src/lib.rs", 18)];
    let dirs = vec![directive(
        "src/lib.rs",
        17,
        "UNI-099",
        18,
        Some("explicit unmatched rule id reference"),
    )];
    let outcome = apply(&mut findings, &dirs, &[], 100);
    assert!(outcome.synthetics.is_empty());
}

/// Test 6: two directives on consecutive lines composing onto the
/// same target line — each targets a different rule and stamps
/// its own finding.
#[test]
fn composed_directives_stamp_two() {
    let mut findings =
        vec![finding("UNI-014", "src/lib.rs", 20), finding("UNI-015", "src/lib.rs", 20)];
    let dirs = vec![
        directive("src/lib.rs", 18, "UNI-014", 20, Some("first rationale long enough text")),
        directive("src/lib.rs", 19, "UNI-015", 20, Some("second rationale long enough text")),
    ];
    let outcome = apply(&mut findings, &dirs, &validation_rules(), 100);
    assert!(outcome.synthetics.is_empty(), "no synthetic expected; got {:?}", outcome.synthetics);
    assert_eq!(findings[0].status, Some(FindingStatus::Ignored));
    assert_eq!(findings[1].status, Some(FindingStatus::Ignored));
    let p0 = findings[0].disposition.as_ref().unwrap().directive.as_ref().unwrap();
    let p1 = findings[1].disposition.as_ref().unwrap().directive.as_ref().unwrap();
    assert_eq!(p0.line, 18);
    assert_eq!(p1.line, 19);
}

/// Test 7: a directive whose path differs from the finding's path
/// never matches, even when the line and rule id agree. The
/// directive surfaces as an orphan.
#[test]
fn directive_path_mismatch_is_orphan() {
    let mut findings = vec![finding("UNI-014", "src/file_b.rs", 18)];
    let dirs = vec![directive(
        "src/file_a.rs",
        17,
        "UNI-014",
        18,
        Some("rationale long enough to clear the floor"),
    )];
    let outcome = apply(&mut findings, &dirs, &validation_rules(), 100);
    assert!(findings[0].status.is_none(), "cross-file finding must not be stamped");
    let ids: Vec<&str> = outcome.synthetics.iter().filter_map(|f| f.rule_id.as_deref()).collect();
    assert_eq!(ids, vec![UNI_023]);
}

/// Determinism: identical inputs in shuffled order produce
/// identical synthetic output (ordering + content).
#[test]
fn apply_output_order_independent() {
    let dirs_a = vec![
        directive("a.rs", 5, "UNI-099", 6, Some("rationale long enough to clear")),
        directive("b.rs", 7, "UNI-100", 8, None),
    ];
    let dirs_b = vec![dirs_a[1].clone(), dirs_a[0].clone()];

    let mut findings_a: Vec<Diagnostic> = Vec::new();
    let mut findings_b: Vec<Diagnostic> = Vec::new();
    let resolved = validation_rules();
    let out_a = apply(&mut findings_a, &dirs_a, &resolved, 1);
    let out_b = apply(&mut findings_b, &dirs_b, &resolved, 1);
    assert_eq!(out_a, out_b);
}

/// Test 8: status-aware exit helper — only `Open` (or unset)
/// findings with `critical | important` severity block.
#[test]
fn blocking_present_respects_status() {
    // Open + Important → blocks.
    let mut open_important = finding("UNI-014", "src/lib.rs", 1);
    open_important.severity = Severity::Important;
    open_important.status = Some(FindingStatus::Open);
    assert!(blocking_findings_present(std::slice::from_ref(&open_important)));

    // Unset status + Critical → blocks (treated as open).
    let mut unset_critical = finding("UNI-014", "src/lib.rs", 2);
    unset_critical.severity = Severity::Critical;
    unset_critical.status = None;
    assert!(blocking_findings_present(std::slice::from_ref(&unset_critical)));

    // Ignored + Critical → does NOT block.
    let mut ignored_critical = finding("UNI-014", "src/lib.rs", 3);
    ignored_critical.severity = Severity::Critical;
    ignored_critical.status = Some(FindingStatus::Ignored);
    assert!(!blocking_findings_present(std::slice::from_ref(&ignored_critical)));

    // FalsePositive + Important → does NOT block.
    let mut fp_important = finding("UNI-014", "src/lib.rs", 4);
    fp_important.severity = Severity::Important;
    fp_important.status = Some(FindingStatus::FalsePositive);
    assert!(!blocking_findings_present(std::slice::from_ref(&fp_important)));

    // Open + Suggestion → does NOT block (severity below the
    // blocking threshold).
    let mut open_suggestion = finding("UNI-014", "src/lib.rs", 5);
    open_suggestion.severity = Severity::Suggestion;
    open_suggestion.status = Some(FindingStatus::Open);
    assert!(!blocking_findings_present(std::slice::from_ref(&open_suggestion)));

    // Mixed set with one open-critical hidden among ignored ones:
    // still blocks.
    let set = vec![ignored_critical.clone(), open_important, open_suggestion];
    assert!(blocking_findings_present(&set));

    // All ignored / false-positive: no blocking.
    let safe_set = vec![ignored_critical, fp_important];
    assert!(!blocking_findings_present(&safe_set));

    // Empty: no blocking.
    assert!(!blocking_findings_present(&[]));
}

fn report(findings: Vec<Diagnostic>) -> DiagnosticReport {
    DiagnosticReport {
        version: DiagnosticReportVersion,
        summary: DiagnosticSummary::from_diagnostics(&findings),
        findings,
    }
}

fn blocking_finding(severity: Severity, status: Option<FindingStatus>) -> Diagnostic {
    let mut f = finding("UNI-014", "src/lib.rs", 1);
    f.severity = severity;
    f.status = status;
    f
}

/// The `Result`-returning gate wraps [`blocking_findings_present`]:
/// non-blocking sets pass; an open critical/important set returns
/// `Error::Validation { review-findings-present }`.
#[test]
fn deny_blocking_maps_to_validation() {
    // Empty / ignored / false-positive / sub-threshold → Ok.
    deny_blocking_findings(&report(vec![])).expect("empty envelope must exit 0");
    deny_blocking_findings(&report(vec![blocking_finding(
        Severity::Critical,
        Some(FindingStatus::Ignored),
    )]))
    .expect("ignored critical must not block");
    deny_blocking_findings(&report(vec![blocking_finding(
        Severity::Important,
        Some(FindingStatus::FalsePositive),
    )]))
    .expect("false-positive important must not block");
    deny_blocking_findings(&report(vec![blocking_finding(
        Severity::Suggestion,
        Some(FindingStatus::Open),
    )]))
    .expect("suggestion severity must not block");

    // Open critical → Err with the stable code.
    let err = deny_blocking_findings(&report(vec![blocking_finding(
        Severity::Critical,
        Some(FindingStatus::Open),
    )]))
    .expect_err("open critical blocks");
    match err {
        Error::Validation { code, .. } => assert_eq!(code, "review-findings-present"),
        other => panic!("expected Validation, got {other:?}"),
    }

    // Unset status + important → treated as open → Err.
    let err = deny_blocking_findings(&report(vec![blocking_finding(Severity::Important, None)]))
        .expect_err("unset status treated as open");
    assert!(matches!(err, Error::Validation { .. }));

    // Mixed: one open important hidden among non-blocking → Err.
    let mixed = vec![
        blocking_finding(Severity::Critical, Some(FindingStatus::Ignored)),
        blocking_finding(Severity::Important, Some(FindingStatus::FalsePositive)),
        blocking_finding(Severity::Important, Some(FindingStatus::Open)),
    ];
    assert!(matches!(deny_blocking_findings(&report(mixed)), Err(Error::Validation { .. })));
}
