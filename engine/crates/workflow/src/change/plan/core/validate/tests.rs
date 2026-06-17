use std::collections::{BTreeMap, HashSet};

use specify_diagnostics::{Severity, blocking};
use specify_model::evidence::ClaimKind;
use tempfile::tempdir;

use super::super::model::{
    Disagreement, DisagreementValue, Divergence, Plan, SliceAuthorityOverride, SliceSourceBinding,
    SourceBinding, Status,
};
use super::super::{PLAN_EXAMPLE_YAML, change, plan_with_changes};
use crate::change::{CYCLE, detect};
use crate::registry::{Registry, RegistryProject};

/// Match a neutral diagnostic on its stable check code (`rule_id`).
fn has_code(d: &specify_diagnostics::Diagnostic, code: &str) -> bool {
    d.rule_id.as_deref() == Some(code)
}

fn disagreement(field: &str, values: &[(&str, &str)]) -> Disagreement {
    Disagreement {
        field: field.to_string(),
        values: values
            .iter()
            .map(|(source, value)| DisagreementValue {
                source: (*source).to_string(),
                value: (*value).to_string(),
            })
            .collect(),
    }
}

fn reg_project(name: &str, adapter: &str) -> RegistryProject {
    RegistryProject {
        name: name.to_string(),
        url: ".".to_string(),
        adapter: Some(adapter.to_string()),
        description: None,
        contracts: None,
        greenfield_seed: None,
    }
}

fn registry(projects: Vec<RegistryProject>) -> Registry {
    Registry { version: 1, projects }
}

#[test]
fn clean_plan_validates() {
    let plan: Plan = serde_saphyr::from_str(PLAN_EXAMPLE_YAML).expect("parse plan fixture");
    let results = plan.validate(None, None);
    assert!(
        results.is_empty(),
        "expected a clean fixture to validate with no findings, got: {results:#?}"
    );
}

#[test]
fn duplicate_name_error() {
    let plan = plan_with_changes(vec![change("foo", Status::Done), change("foo", Status::Pending)]);
    let results = plan.validate(None, None);
    let dupes: Vec<_> = results.iter().filter(|r| has_code(r, "duplicate-name")).collect();
    assert_eq!(dupes.len(), 1, "expected one duplicate-name result, got {results:#?}");
    assert_eq!(dupes[0].severity, Severity::Important);
    assert_eq!(dupes[0].slice.as_deref(), Some("foo"));
}

/// The divergence-coherence quartet: a flag with no recorded multi-source
/// disagreement (or only a single-source value) is `slice-divergence-unrecorded`;
/// a genuine two-source disagreement is consistent; disagreements without a
/// flag are advisory `slice-divergence-orphan-values`. All are non-blocking.
#[test]
fn divergence_checks() {
    let mut a = change("a", Status::Pending);
    a.divergence = Some(Divergence::Likely);
    let plan = plan_with_changes(vec![a]);
    let results = plan.validate(None, None);
    let hits: Vec<_> =
        results.iter().filter(|r| has_code(r, "slice-divergence-unrecorded")).collect();
    assert_eq!(hits.len(), 1, "flag-only is unrecorded: {results:#?}");
    assert_eq!(hits[0].severity, Severity::Suggestion);
    assert!(!blocking(hits[0]), "divergence is operator-settable standalone; must not block");

    let mut a = change("a", Status::Pending);
    a.divergence = Some(Divergence::Likely);
    a.disagreements = vec![disagreement("min-length", &[("docs", "8")])];
    let plan = plan_with_changes(vec![a]);
    assert!(
        plan.validate(None, None).iter().any(|r| has_code(r, "slice-divergence-unrecorded")),
        "a single source value is not a disagreement"
    );

    let mut a = change("a", Status::Pending);
    a.divergence = Some(Divergence::Likely);
    a.disagreements = vec![disagreement("min-length", &[("docs", "8"), ("legacy", "12")])];
    let plan = plan_with_changes(vec![a]);
    assert!(
        !plan.validate(None, None).iter().any(|r| has_code(r, "slice-divergence-unrecorded")),
        "a two-source disagreement is consistent with the flag"
    );

    let mut a = change("a", Status::Pending);
    a.disagreements = vec![disagreement("min-length", &[("docs", "8"), ("legacy", "12")])];
    let plan = plan_with_changes(vec![a]);
    let results = plan.validate(None, None);
    let hits: Vec<_> =
        results.iter().filter(|r| has_code(r, "slice-divergence-orphan-values")).collect();
    assert_eq!(hits.len(), 1, "disagreements without a flag are orphan values: {results:#?}");
    assert_eq!(hits[0].severity, Severity::Suggestion);
    assert!(!blocking(hits[0]), "orphan values are advisory, not blocking");
}

#[test]
fn cycle_detection() {
    // A 3-cycle names all members...
    let mut a = change("a", Status::Pending);
    a.depends_on = vec!["c".into()];
    let mut b = change("b", Status::Pending);
    b.depends_on = vec!["a".into()];
    let mut c = change("c", Status::Pending);
    c.depends_on = vec!["b".into()];
    let plan = plan_with_changes(vec![a, b, c]);
    let cycles: Vec<_> = detect(&plan.entries).into_iter().filter(|d| has_code(d, CYCLE)).collect();
    assert!(!cycles.is_empty(), "expected at least one {CYCLE}, got {cycles:#?}");
    let msg = &cycles[0].impact;
    assert!(
        msg.contains('a') && msg.contains('b') && msg.contains('c'),
        "cycle message should name all members: {msg}"
    );

    // ...and a self-edge is a cycle too.
    let mut a = change("a", Status::Pending);
    a.depends_on = vec!["a".into()];
    let plan = plan_with_changes(vec![a]);
    assert!(
        detect(&plan.entries).iter().any(|d| has_code(d, CYCLE)),
        "expected a {CYCLE} result for a self-edge"
    );
}

#[test]
fn unknown_depends_on_error() {
    let mut entry = change("depends-on-ghost", Status::Pending);
    entry.depends_on = vec!["bogus".into()];
    let plan = plan_with_changes(vec![entry]);
    let results = plan.validate(None, None);
    let hits: Vec<_> = results.iter().filter(|r| has_code(r, "unknown-depends-on")).collect();
    assert_eq!(hits.len(), 1, "expected one unknown-depends-on, got {results:#?}");
    assert_eq!(hits[0].slice.as_deref(), Some("depends-on-ghost"));
    assert!(hits[0].impact.contains("bogus"));
}

#[test]
fn unknown_source_error() {
    let mut entry = change("source-ghost", Status::Pending);
    entry.sources = vec![SliceSourceBinding::bare("monolith")];
    let plan = plan_with_changes(vec![entry]);
    let results = plan.validate(None, None);
    let hits: Vec<_> = results.iter().filter(|r| has_code(r, "unknown-source")).collect();
    assert_eq!(hits.len(), 1, "expected one unknown-source, got {results:#?}");
    assert_eq!(hits[0].slice.as_deref(), Some("source-ghost"));
    assert!(hits[0].impact.contains("monolith"));
}

#[test]
fn source_key_uniqueness() {
    // Two bindings on the same source key block as duplicate-source-key...
    let mut entry = change("doubled", Status::Pending);
    entry.sources = vec![
        SliceSourceBinding::structured("docs", "lead-a"),
        SliceSourceBinding::structured("docs", "lead-b"),
    ];
    let mut plan = plan_with_changes(vec![entry]);
    plan.sources.insert("docs".into(), SourceBinding::path("documentation", "/tmp/docs"));
    let results = plan.validate(None, None);
    let hits: Vec<_> = results.iter().filter(|r| has_code(r, "duplicate-source-key")).collect();
    assert_eq!(hits.len(), 1, "expected one duplicate-source-key, got {results:#?}");
    assert_eq!(hits[0].slice.as_deref(), Some("doubled"));
    assert!(hits[0].impact.contains("docs"));
    assert!(blocking(hits[0]), "duplicate-source-key must block");

    // ...but distinct keys (even sharing a lead) pass.
    let mut entry = change("multi", Status::Pending);
    entry.sources = vec![
        SliceSourceBinding::structured("docs", "lead-a"),
        SliceSourceBinding::structured("legacy", "lead-a"),
    ];
    let mut plan = plan_with_changes(vec![entry]);
    plan.sources.insert("docs".into(), SourceBinding::path("documentation", "/tmp/docs"));
    plan.sources.insert("legacy".into(), SourceBinding::path("typescript", "/tmp/legacy"));
    assert!(
        !plan.validate(None, None).iter().any(|r| has_code(r, "duplicate-source-key")),
        "distinct keys must not trip duplicate-source-key"
    );
}

#[test]
fn in_progress_cardinality() {
    // Two in-progress entries each flag multiple-in-progress...
    let plan = plan_with_changes(vec![
        change("first-in-progress", Status::InProgress),
        change("second-in-progress", Status::InProgress),
    ]);
    let results = plan.validate(None, None);
    let hits: Vec<_> = results.iter().filter(|r| has_code(r, "multiple-in-progress")).collect();
    assert_eq!(hits.len(), 2, "expected one result per offender, got {results:#?}");
    let names: HashSet<&str> = hits.iter().filter_map(|r| r.slice.as_deref()).collect();
    assert!(
        names.contains("first-in-progress") && names.contains("second-in-progress"),
        "names = {names:?}"
    );

    // ...a single in-progress entry is fine.
    let plan = plan_with_changes(vec![
        change("only-in-progress", Status::InProgress),
        change("queued", Status::Pending),
    ]);
    assert!(
        !plan.validate(None, None).iter().any(|r| has_code(r, "multiple-in-progress")),
        "single in-progress entry should not trip multiple-in-progress"
    );
}

/// Slice-directory consistency against a `slices_dir`: an orphan dir warns,
/// a missing dir for an in-progress entry warns, a present dir is silent, and
/// passing `None` skips the checks entirely. All warnings are suggestions.
#[test]
fn slice_dir_consistency() {
    let tmp = tempdir().expect("tempdir");
    std::fs::create_dir(tmp.path().join("stale-slice")).expect("mkdir");
    let plan = plan_with_changes(vec![change("other", Status::Pending)]);
    let results = plan.validate(Some(tmp.path()), None);
    let hits: Vec<_> = results.iter().filter(|r| has_code(r, "orphan-slice-dir")).collect();
    assert_eq!(hits.len(), 1, "expected one orphan-slice-dir, got {results:#?}");
    assert_eq!(hits[0].severity, Severity::Suggestion);
    assert_eq!(hits[0].slice.as_deref(), Some("stale-slice"));

    let tmp = tempdir().expect("tempdir");
    let plan = plan_with_changes(vec![change("alpha", Status::InProgress)]);
    let results = plan.validate(Some(tmp.path()), None);
    let hits: Vec<_> =
        results.iter().filter(|r| has_code(r, "missing-slice-dir-for-in-progress")).collect();
    assert_eq!(hits.len(), 1, "expected one missing-dir warning, got {results:#?}");
    assert_eq!(hits[0].severity, Severity::Suggestion);
    assert_eq!(hits[0].slice.as_deref(), Some("alpha"));

    let tmp = tempdir().expect("tempdir");
    std::fs::create_dir(tmp.path().join("alpha")).expect("mkdir alpha");
    let plan = plan_with_changes(vec![change("alpha", Status::InProgress)]);
    let results = plan.validate(Some(tmp.path()), None);
    assert!(
        !results
            .iter()
            .any(|r| has_code(r, "orphan-slice-dir")
                || has_code(r, "missing-slice-dir-for-in-progress")),
        "a present dir yields no directory warnings, got: {results:#?}"
    );

    let plan = plan_with_changes(vec![change("alpha", Status::InProgress)]);
    let results = plan.validate(None, None);
    assert!(
        !results
            .iter()
            .any(|r| has_code(r, "orphan-slice-dir")
                || has_code(r, "missing-slice-dir-for-in-progress")),
        "None slices_dir must skip directory consistency checks: {results:#?}"
    );
}

#[test]
fn no_short_circuit() {
    let mut a = change("foo", Status::Pending);
    a.depends_on = vec!["missing".into()];
    a.sources = vec![SliceSourceBinding::bare("ghost-source")];
    let b = change("foo", Status::Pending);
    let plan = plan_with_changes(vec![a, b]);
    let results = plan.validate(None, None);

    let codes: HashSet<&str> = results.iter().filter_map(|r| r.rule_id.as_deref()).collect();
    for expected in ["duplicate-name", "unknown-depends-on", "unknown-source"] {
        assert!(
            codes.contains(expected),
            "expected code {expected} in {codes:?} — validate must not short-circuit"
        );
    }
}

/// Project-binding resolution against the registry: an unknown project flags
/// `project-not-in-registry`; a matching project is clean; an omitted project
/// resolves with no registry and with a single-project registry, but a
/// multi-project registry flags `plan-reconcile-project-binding-required`.
#[test]
fn project_binding_checks() {
    let mut e = change("registry-missing", Status::Pending);
    e.project = Some("nonexistent".to_string());
    let plan = plan_with_changes(vec![e]);
    let reg = registry(vec![reg_project("real-project", "omnia@1.0.0")]);
    assert!(plan.validate(None, Some(&reg)).iter().any(|r| has_code(r, "project-not-in-registry")));

    let mut e = change("project-alpha", Status::Pending);
    e.project = Some("alpha".to_string());
    let plan = plan_with_changes(vec![e]);
    let reg =
        registry(vec![reg_project("alpha", "omnia@1.0.0"), reg_project("beta", "omnia@1.0.0")]);
    assert!(!plan.validate(None, Some(&reg)).iter().any(blocking));

    let mut e = change("orphan", Status::Pending);
    e.project = None;
    let plan = plan_with_changes(vec![e]);
    assert!(
        !plan.validate(None, None).iter().any(blocking),
        "an omitted project must validate cleanly without a registry"
    );

    let mut e = change("solo", Status::Pending);
    e.project = None;
    let plan = plan_with_changes(vec![e]);
    let reg = registry(vec![reg_project("only", "omnia@1.0.0")]);
    assert!(
        !plan
            .validate(None, Some(&reg))
            .iter()
            .any(|r| has_code(r, "plan-reconcile-project-binding-required")),
        "a single-project registry must auto-resolve an omitted project"
    );

    let mut e = change("ambiguous", Status::Pending);
    e.project = None;
    let plan = plan_with_changes(vec![e]);
    let reg =
        registry(vec![reg_project("alpha", "omnia@1.0.0"), reg_project("beta", "contracts@1.0.0")]);
    assert!(
        plan.validate(None, Some(&reg))
            .iter()
            .any(|r| has_code(r, "plan-reconcile-project-binding-required") && blocking(r)),
        "a multi-project registry must flag an omitted project"
    );
}

#[test]
fn context_path_validation() {
    // `..` is rejected...
    let mut entry = change("foo", Status::Pending);
    entry.context = vec!["../etc/passwd".into()];
    let plan = plan_with_changes(vec![entry]);
    let errors: Vec<_> = plan
        .validate(None, None)
        .into_iter()
        .filter(|r| has_code(r, "plan.context-path-invalid"))
        .collect();
    assert_eq!(errors.len(), 1, "expected one context-path-invalid for `..`");
    assert!(errors[0].impact.contains(".."), "message should mention '..'");

    // ...an absolute path is rejected...
    let mut entry = change("foo", Status::Pending);
    entry.context = vec!["/absolute/path".into()];
    let plan = plan_with_changes(vec![entry]);
    let errors: Vec<_> = plan
        .validate(None, None)
        .into_iter()
        .filter(|r| has_code(r, "plan.context-path-invalid"))
        .collect();
    assert_eq!(errors.len(), 1, "expected one context-path-invalid for an absolute path");
    assert!(errors[0].impact.contains("/absolute/path"));

    // ...and valid relative paths pass.
    let mut entry = change("foo", Status::Pending);
    entry.context =
        vec!["contracts/http/user-api.yaml".into(), "specs/user-registration/spec.md".into()];
    let plan = plan_with_changes(vec![entry]);
    assert!(
        !plan.validate(None, None).into_iter().any(|r| has_code(&r, "plan.context-path-invalid")),
        "valid relative paths must not produce errors"
    );
}

/// `slice-authority-override-orphan-source`: an override naming an unbound
/// source is flagged (with kind + bad key in the message); empty and
/// all-valid maps pass; multiple orphans sort by `ClaimKind` declaration
/// order (requirement, criterion, decision).
#[test]
fn authority_override_checks() {
    let mut entry = change("identity-user-registration", Status::Pending);
    entry.sources = vec![SliceSourceBinding::bare("legacy")];
    entry.authority_override = SliceAuthorityOverride {
        by_kind: BTreeMap::from([
            (ClaimKind::Requirement, "phantom".to_string()),
            (ClaimKind::Criterion, "legacy".to_string()),
        ]),
    };
    let mut plan = plan_with_changes(vec![entry]);
    plan.sources.insert("legacy".into(), SourceBinding::path("typescript", "/tmp"));
    let hits: Vec<_> = plan
        .validate(None, None)
        .into_iter()
        .filter(|r| has_code(r, "slice-authority-override-orphan-source"))
        .collect();
    assert_eq!(hits.len(), 1, "expected one orphan finding, got: {hits:#?}");
    assert_eq!(hits[0].slice.as_deref(), Some("identity-user-registration"));
    assert!(
        hits[0].impact.contains("requirement") && hits[0].impact.contains("phantom"),
        "message must name kind + bad source key, got: {}",
        hits[0].impact
    );

    let mut entry = change("any", Status::Pending);
    entry.sources = vec![SliceSourceBinding::bare("legacy")];
    let mut plan = plan_with_changes(vec![entry]);
    plan.sources.insert("legacy".into(), SourceBinding::path("typescript", "/tmp"));
    assert!(
        !plan
            .validate(None, None)
            .iter()
            .any(|r| has_code(r, "slice-authority-override-orphan-source")),
        "empty override map must not trip orphan check"
    );

    let mut entry = change("any", Status::Pending);
    entry.sources = vec![SliceSourceBinding::bare("legacy"), SliceSourceBinding::bare("runtime")];
    entry.authority_override = SliceAuthorityOverride {
        by_kind: BTreeMap::from([
            (ClaimKind::Requirement, "runtime".to_string()),
            (ClaimKind::Criterion, "legacy".to_string()),
        ]),
    };
    let mut plan = plan_with_changes(vec![entry]);
    plan.sources.insert("legacy".into(), SourceBinding::path("typescript", "/tmp/legacy"));
    plan.sources.insert("runtime".into(), SourceBinding::path("captures", "/tmp/runtime"));
    assert!(
        !plan
            .validate(None, None)
            .iter()
            .any(|r| has_code(r, "slice-authority-override-orphan-source")),
        "all-valid overrides must pass"
    );

    let mut entry = change("identity-user-registration", Status::Pending);
    entry.sources = vec![SliceSourceBinding::bare("legacy")];
    entry.authority_override = SliceAuthorityOverride {
        by_kind: BTreeMap::from([
            (ClaimKind::Requirement, "ghost-a".to_string()),
            (ClaimKind::Criterion, "ghost-b".to_string()),
            (ClaimKind::Decision, "ghost-c".to_string()),
        ]),
    };
    let mut plan = plan_with_changes(vec![entry]);
    plan.sources.insert("legacy".into(), SourceBinding::path("typescript", "/tmp"));
    let codes: Vec<&str> = plan
        .validate(None, None)
        .iter()
        .filter(|r| has_code(r, "slice-authority-override-orphan-source"))
        .map(|r| {
            // Pull the kind out of the message (between "kind '" and "'").
            let msg = &r.impact;
            let start = msg.find("kind '").unwrap() + "kind '".len();
            let end = start + msg[start..].find('\'').unwrap();
            &msg[start..end]
        })
        .map(|s| -> &'static str {
            match s {
                "requirement" => "requirement",
                "criterion" => "criterion",
                "decision" => "decision",
                _ => "other",
            }
        })
        .collect();
    assert_eq!(codes, vec!["requirement", "criterion", "decision"]);
}
