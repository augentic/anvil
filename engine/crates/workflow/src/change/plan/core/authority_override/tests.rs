use specify_model::evidence::ClaimKind;

use super::super::model::{SliceSourceBinding, Status};
use super::super::{change, plan_with_changes};
use super::*;

fn ts() -> jiff::Timestamp {
    "2026-06-02T00:00:00Z".parse().expect("timestamp")
}

/// `(action, claim-kind)` projection of one journal event, dropping
/// the timestamp so ordering and shape assertions stay readable.
fn shape(event: &journal::Event) -> (AuthorityOverrideAction, Option<String>, Option<String>) {
    match &event.kind {
        journal::EventKind::PlanAmendAuthorityOverride {
            action,
            claim_kind,
            source,
            ..
        } => (*action, claim_kind.clone(), source.clone()),
        other => panic!("expected PlanAmendAuthorityOverride, got {other:?}"),
    }
}

#[test]
fn set_then_clear_resolves_cleared() {
    let mut plan = plan_with_changes(vec![change("foo", Status::Pending)]);
    let events = mutate_authority_overrides(
        &mut plan,
        "p",
        &[("foo".into(), ClaimKind::Requirement, "src-a".into())],
        &[("foo".into(), ClaimKind::Requirement)],
        &[],
        ts(),
    )
    .expect("mutate ok");

    assert!(
        plan.entries[0].authority_override.by_kind.is_empty(),
        "set then clear on the same (slice, kind) must resolve cleared"
    );
    assert_eq!(events.len(), 1, "suppressed set leaves only the clear event");
    assert_eq!(
        shape(&events[0]),
        (AuthorityOverrideAction::Clear, Some("requirement".into()), None),
        "journal records the clear, not the superseded set"
    );
}

#[test]
fn set_dedup_collapses_pair() {
    let mut plan = plan_with_changes(vec![change("foo", Status::Pending)]);
    let events = mutate_authority_overrides(
        &mut plan,
        "p",
        &[
            ("foo".into(), ClaimKind::Requirement, "src-a".into()),
            ("foo".into(), ClaimKind::Requirement, "src-b".into()),
        ],
        &[],
        &[],
        ts(),
    )
    .expect("mutate ok");

    assert_eq!(
        plan.entries[0].authority_override.by_kind.get(&ClaimKind::Requirement).map(String::as_str),
        Some("src-b"),
        "duplicate (slice, kind) collapses to the last value"
    );
    assert_eq!(events.len(), 1, "deduped pair emits a single set event");
    assert_eq!(
        shape(&events[0]),
        (AuthorityOverrideAction::Set, Some("requirement".into()), Some("src-b".into())),
    );
}

#[test]
fn set_distinct_kinds_coexist() {
    let mut plan = plan_with_changes(vec![change("foo", Status::Pending)]);
    mutate_authority_overrides(
        &mut plan,
        "p",
        &[
            ("foo".into(), ClaimKind::Requirement, "src-a".into()),
            ("foo".into(), ClaimKind::Decision, "src-b".into()),
        ],
        &[],
        &[],
        ts(),
    )
    .expect("mutate ok");

    let by_kind = &plan.entries[0].authority_override.by_kind;
    assert_eq!(by_kind.len(), 2, "distinct kinds coexist on one slice");
    assert_eq!(by_kind.get(&ClaimKind::Requirement).map(String::as_str), Some("src-a"));
    assert_eq!(by_kind.get(&ClaimKind::Decision).map(String::as_str), Some("src-b"));
}

#[test]
fn orphan_source_rejected() {
    let mut entry = change("foo", Status::Pending);
    entry.sources = vec![SliceSourceBinding::bare("bound")];
    let mut plan = plan_with_changes(vec![entry]);

    mutate_authority_overrides(
        &mut plan,
        "p",
        &[("foo".into(), ClaimKind::Requirement, "unbound".into())],
        &[],
        &[],
        ts(),
    )
    .expect("mutate writes the override unconditionally");

    let err = reject_orphan_overrides(&plan).expect_err("orphan source must be rejected");
    match err {
        Error::Validation { code, .. } => {
            assert_eq!(code, "slice-authority-override-orphan-source");
        }
        other => panic!("expected Error::Validation, got {other:?}"),
    }
}

#[test]
fn bound_source_accepted() {
    let mut entry = change("foo", Status::Pending);
    entry.sources = vec![SliceSourceBinding::bare("bound")];
    let mut plan = plan_with_changes(vec![entry]);

    mutate_authority_overrides(
        &mut plan,
        "p",
        &[("foo".into(), ClaimKind::Requirement, "bound".into())],
        &[],
        &[],
        ts(),
    )
    .expect("mutate ok");

    reject_orphan_overrides(&plan).expect("override naming a bound source must pass the gate");
}

#[test]
fn unknown_slice_refused() {
    let mut plan = plan_with_changes(vec![change("foo", Status::Pending)]);
    let err = mutate_authority_overrides(
        &mut plan,
        "p",
        &[("ghost".into(), ClaimKind::Requirement, "src-a".into())],
        &[],
        &[],
        ts(),
    )
    .expect_err("set on an absent slice must Err before any mutation");
    match err {
        Error::Validation { code, .. } => {
            assert_eq!(code, "plan-authority-override-unknown-slice");
        }
        other => panic!("expected Error::Validation, got {other:?}"),
    }
    assert!(
        plan.entries[0].authority_override.by_kind.is_empty(),
        "pre-flight refusal leaves the plan untouched"
    );
}

#[test]
fn events_sort_by_kind_then_action() {
    let mut entry = change("foo", Status::Pending);
    entry.authority_override.by_kind.insert(ClaimKind::Criterion, "seed".into());
    let mut plan = plan_with_changes(vec![entry]);

    // Sets are issued out of kind order to prove the output sort is
    // independent of operator-issued flag order.
    let events = mutate_authority_overrides(
        &mut plan,
        "p",
        &[
            ("foo".into(), ClaimKind::Requirement, "r".into()),
            ("foo".into(), ClaimKind::Intent, "i".into()),
        ],
        &[("foo".into(), ClaimKind::Criterion)],
        &[],
        ts(),
    )
    .expect("mutate ok");

    let shapes: Vec<_> = events.iter().map(shape).collect();
    assert_eq!(
        shapes,
        vec![
            (AuthorityOverrideAction::Clear, Some("criterion".into()), None),
            (AuthorityOverrideAction::Set, Some("intent".into()), Some("i".into())),
            (AuthorityOverrideAction::Set, Some("requirement".into()), Some("r".into())),
        ],
        "events sort by (slice, kind, action) regardless of input order"
    );
}

#[test]
fn clear_all_emits_per_kind() {
    let mut entry = change("foo", Status::Pending);
    entry.authority_override.by_kind.insert(ClaimKind::Decision, "d".into());
    entry.authority_override.by_kind.insert(ClaimKind::Requirement, "r".into());
    let mut plan = plan_with_changes(vec![entry]);

    let events = mutate_authority_overrides(&mut plan, "p", &[], &[], &["foo".into()], ts())
        .expect("mutate ok");

    assert!(
        plan.entries[0].authority_override.by_kind.is_empty(),
        "whole-map clear wipes every kind"
    );
    let shapes: Vec<_> = events.iter().map(shape).collect();
    assert_eq!(
        shapes,
        vec![
            (AuthorityOverrideAction::Clear, Some("decision".into()), None),
            (AuthorityOverrideAction::Clear, Some("requirement".into()), None),
        ],
        "whole-map clear emits one Clear per kind present before the wipe"
    );
}

#[test]
fn plan_and_slice_names_threaded() {
    let mut plan = plan_with_changes(vec![change("foo", Status::Pending)]);
    let events = mutate_authority_overrides(
        &mut plan,
        "platform-v2",
        &[("foo".into(), ClaimKind::Requirement, "src-a".into())],
        &[],
        &[],
        ts(),
    )
    .expect("mutate ok");

    match &events[0].kind {
        journal::EventKind::PlanAmendAuthorityOverride {
            plan_name,
            slice_name,
            ..
        } => {
            assert_eq!(plan_name.as_str(), "platform-v2");
            assert_eq!(slice_name.as_str(), "foo");
        }
        other => panic!("expected PlanAmendAuthorityOverride, got {other:?}"),
    }
}
