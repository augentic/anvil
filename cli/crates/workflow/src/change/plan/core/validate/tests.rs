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

#[test]
fn divergence_flag_no_values_advisory() {
    let mut a = change("a", Status::Pending);
    a.divergence = Some(Divergence::Likely);
    let plan = plan_with_changes(vec![a]);
    let results = plan.validate(None, None);
    let hits: Vec<_> =
        results.iter().filter(|r| has_code(r, "slice-divergence-unrecorded")).collect();
    assert_eq!(hits.len(), 1, "got {results:#?}");
    assert_eq!(hits[0].severity, Severity::Suggestion);
    assert!(!blocking(hits[0]), "divergence is operator-settable standalone; must not block");
}

#[test]
fn divergence_single_source_flagged() {
    let mut a = change("a", Status::Pending);
    a.divergence = Some(Divergence::Likely);
    a.disagreements = vec![disagreement("min-length", &[("docs", "8")])];
    let plan = plan_with_changes(vec![a]);
    let results = plan.validate(None, None);
    assert!(
        results.iter().any(|r| has_code(r, "slice-divergence-unrecorded")),
        "a single source value is not a disagreement: {results:#?}"
    );
}

#[test]
fn divergence_two_sources_clean() {
    let mut a = change("a", Status::Pending);
    a.divergence = Some(Divergence::Likely);
    a.disagreements = vec![disagreement("min-length", &[("docs", "8"), ("legacy", "12")])];
    let plan = plan_with_changes(vec![a]);
    let results = plan.validate(None, None);
    assert!(
        !results.iter().any(|r| has_code(r, "slice-divergence-unrecorded")),
        "a two-source disagreement is consistent: {results:#?}"
    );
}

#[test]
fn disagreements_without_flag_is_advisory() {
    let mut a = change("a", Status::Pending);
    a.disagreements = vec![disagreement("min-length", &[("docs", "8"), ("legacy", "12")])];
    let plan = plan_with_changes(vec![a]);
    let results = plan.validate(None, None);
    let hits: Vec<_> =
        results.iter().filter(|r| has_code(r, "slice-divergence-orphan-values")).collect();
    assert_eq!(hits.len(), 1, "got {results:#?}");
    assert_eq!(hits[0].severity, Severity::Suggestion);
    assert!(!blocking(hits[0]), "orphan values are advisory, not blocking");
}

#[test]
fn cycle_error() {
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
    assert!(msg.contains('a'), "cycle message should name a: {msg}");
    assert!(msg.contains('b'), "cycle message should name b: {msg}");
    assert!(msg.contains('c'), "cycle message should name c: {msg}");
}

#[test]
fn self_cycle_error() {
    let mut a = change("a", Status::Pending);
    a.depends_on = vec!["a".into()];
    let plan = plan_with_changes(vec![a]);
    let cycles = detect(&plan.entries);
    assert!(
        cycles.iter().any(|d| has_code(d, CYCLE)),
        "expected a {CYCLE} result for self-edge, got: {cycles:#?}"
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
fn duplicate_source_key_error() {
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
}

#[test]
fn distinct_source_keys_pass() {
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
fn multiple_in_progress_error() {
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
}

#[test]
fn single_in_progress_is_fine() {
    let plan = plan_with_changes(vec![
        change("only-in-progress", Status::InProgress),
        change("queued", Status::Pending),
    ]);
    let results = plan.validate(None, None);
    assert!(
        !results.iter().any(|r| has_code(r, "multiple-in-progress")),
        "single in-progress entry should not trip multiple-in-progress: {results:#?}"
    );
}

#[test]
fn orphan_dir_warning() {
    let tmp = tempdir().expect("tempdir");
    std::fs::create_dir(tmp.path().join("stale-slice")).expect("mkdir");
    let plan = plan_with_changes(vec![change("other", Status::Pending)]);
    let results = plan.validate(Some(tmp.path()), None);
    let hits: Vec<_> = results.iter().filter(|r| has_code(r, "orphan-slice-dir")).collect();
    assert_eq!(hits.len(), 1, "expected one orphan-slice-dir, got {results:#?}");
    assert_eq!(hits[0].severity, Severity::Suggestion);
    assert_eq!(hits[0].slice.as_deref(), Some("stale-slice"));
}

#[test]
fn missing_dir_for_in_progress_warning() {
    let tmp = tempdir().expect("tempdir");
    let plan = plan_with_changes(vec![change("alpha", Status::InProgress)]);
    let results = plan.validate(Some(tmp.path()), None);
    let hits: Vec<_> =
        results.iter().filter(|r| has_code(r, "missing-slice-dir-for-in-progress")).collect();
    assert_eq!(hits.len(), 1, "expected one missing-dir warning, got {results:#?}");
    assert_eq!(hits[0].severity, Severity::Suggestion);
    assert_eq!(hits[0].slice.as_deref(), Some("alpha"));
}

#[test]
fn present_dir_for_in_progress_silent() {
    let tmp = tempdir().expect("tempdir");
    std::fs::create_dir(tmp.path().join("alpha")).expect("mkdir alpha");
    let plan = plan_with_changes(vec![change("alpha", Status::InProgress)]);
    let results = plan.validate(Some(tmp.path()), None);
    assert!(
        !results
            .iter()
            .any(|r| has_code(r, "orphan-slice-dir")
                || has_code(r, "missing-slice-dir-for-in-progress")),
        "no directory warnings expected, got: {results:#?}"
    );
}

#[test]
fn no_slices_dir_skips_consistency() {
    let plan = plan_with_changes(vec![change("alpha", Status::InProgress)]);
    let results = plan.validate(None, None);
    assert!(
        !results
            .iter()
            .any(|r| has_code(r, "orphan-slice-dir")
                || has_code(r, "missing-slice-dir-for-in-progress")),
        "passing None for slices_dir must skip directory consistency checks: {results:#?}"
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

#[test]
fn project_not_in_registry() {
    let mut e = change("registry-missing", Status::Pending);
    e.project = Some("nonexistent".to_string());
    let plan = plan_with_changes(vec![e]);
    let registry = Registry {
        version: 1,
        projects: vec![RegistryProject {
            name: "real-project".to_string(),
            url: ".".to_string(),
            adapter: Some("omnia@1.0.0".to_string()),
            description: None,
            contracts: None,
            greenfield_seed: None,
        }],
    };
    let results = plan.validate(None, Some(&registry));
    assert!(results.iter().any(|r| has_code(r, "project-not-in-registry")));
}

#[test]
fn project_matches_registry() {
    let mut e = change("project-alpha", Status::Pending);
    e.project = Some("alpha".to_string());
    let plan = plan_with_changes(vec![e]);
    let registry = Registry {
        version: 1,
        projects: vec![
            RegistryProject {
                name: "alpha".to_string(),
                url: ".".to_string(),
                adapter: Some("omnia@1.0.0".to_string()),
                description: Some("Alpha".to_string()),
                contracts: None,
                greenfield_seed: None,
            },
            RegistryProject {
                name: "beta".to_string(),
                url: "git@github.com:org/beta.git".to_string(),
                adapter: Some("omnia@1.0.0".to_string()),
                description: Some("Beta".to_string()),
                contracts: None,
                greenfield_seed: None,
            },
        ],
    };
    let results = plan.validate(None, Some(&registry));
    assert!(!results.iter().any(blocking));
}

#[test]
fn omitted_project_passes_without_registry() {
    // A single regular project (no registry) synthesises the sole
    // topology project, so an omitted `project` resolves and must not
    // produce a finding.
    let mut e = change("orphan", Status::Pending);
    e.project = None;
    let plan = plan_with_changes(vec![e]);
    let results = plan.validate(None, None);
    assert!(
        !results.iter().any(blocking),
        "an omitted project must validate cleanly without a registry, got: {results:#?}"
    );
}

#[test]
fn omitted_project_flagged_multi() {
    let mut e = change("ambiguous", Status::Pending);
    e.project = None;
    let plan = plan_with_changes(vec![e]);
    let registry = Registry {
        version: 1,
        projects: vec![
            RegistryProject {
                name: "alpha".to_string(),
                url: ".".to_string(),
                adapter: Some("omnia@1.0.0".to_string()),
                description: None,
                contracts: None,
                greenfield_seed: None,
            },
            RegistryProject {
                name: "beta".to_string(),
                url: "git@github.com:org/beta.git".to_string(),
                adapter: Some("contracts@1.0.0".to_string()),
                description: None,
                contracts: None,
                greenfield_seed: None,
            },
        ],
    };
    let results = plan.validate(None, Some(&registry));
    assert!(
        results
            .iter()
            .any(|r| has_code(r, "plan-reconcile-project-binding-required") && blocking(r)),
        "a multi-project registry must flag an omitted project, got: {results:#?}"
    );
}

#[test]
fn omitted_project_ok_single() {
    let mut e = change("solo", Status::Pending);
    e.project = None;
    let plan = plan_with_changes(vec![e]);
    let registry = Registry {
        version: 1,
        projects: vec![RegistryProject {
            name: "only".to_string(),
            url: ".".to_string(),
            adapter: Some("omnia@1.0.0".to_string()),
            description: None,
            contracts: None,
            greenfield_seed: None,
        }],
    };
    let results = plan.validate(None, Some(&registry));
    assert!(
        !results.iter().any(|r| has_code(r, "plan-reconcile-project-binding-required")),
        "a single-project registry must auto-resolve an omitted project, got: {results:#?}"
    );
}

#[test]
fn context_rejects_dotdot() {
    let mut entry = change("foo", Status::Pending);
    entry.context = vec!["../etc/passwd".into()];
    let plan = plan_with_changes(vec![entry]);
    let errors: Vec<_> = plan
        .validate(None, None)
        .into_iter()
        .filter(|r| has_code(r, "plan.context-path-invalid"))
        .collect();
    assert_eq!(errors.len(), 1, "expected exactly one context-path-invalid error");
    assert!(errors[0].impact.contains(".."), "message should mention '..'");
}

#[test]
fn context_rejects_absolute() {
    let mut entry = change("foo", Status::Pending);
    entry.context = vec!["/absolute/path".into()];
    let plan = plan_with_changes(vec![entry]);
    let errors: Vec<_> = plan
        .validate(None, None)
        .into_iter()
        .filter(|r| has_code(r, "plan.context-path-invalid"))
        .collect();
    assert_eq!(errors.len(), 1, "expected exactly one context-path-invalid error");
    assert!(errors[0].impact.contains("/absolute/path"));
}

#[test]
fn override_orphan_key_rejected() {
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
}

#[test]
fn authority_override_empty_passes() {
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
}

#[test]
fn authority_override_valid_keys_pass() {
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
}

#[test]
fn authority_overrides_sort() {
    let mut entry = change("identity-user-registration", Status::Pending);
    entry.sources = vec![SliceSourceBinding::bare("legacy")];
    // Insert in non-sorted order; BTreeMap iteration sorts by kind.
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
    // ClaimKind PartialOrd matches enum declaration order: Intent,
    // Requirement, Criterion, Decision, …
    assert_eq!(codes, vec!["requirement", "criterion", "decision"]);
}

#[test]
fn context_accepts_valid() {
    let mut entry = change("foo", Status::Pending);
    entry.context =
        vec!["contracts/http/user-api.yaml".into(), "specs/user-registration/spec.md".into()];
    let plan = plan_with_changes(vec![entry]);
    assert!(
        !plan.validate(None, None).into_iter().any(|r| has_code(&r, "plan.context-path-invalid")),
        "valid relative paths must not produce errors"
    );
}
