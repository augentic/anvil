use super::super::{PLAN_EXAMPLE_YAML, change, change_with_deps, plan_with_changes};
use super::*;

#[test]
fn next_eligible_picks_first_ready() {
    let plan = plan_with_changes(vec![
        change("a", Status::Done),
        change("b", Status::Done),
        change_with_deps("c", Status::Pending, &["b"]),
    ]);
    let eligible = plan.next_eligible().expect("c should be eligible");
    assert_eq!(eligible.name, "c");
}

#[test]
fn next_eligible_skips_unmet_deps() {
    let plan = plan_with_changes(vec![
        change("a", Status::Pending),
        change_with_deps("b", Status::Pending, &["a"]),
    ]);
    let eligible = plan.next_eligible().expect("a should be eligible");
    assert_eq!(eligible.name, "a", "b's dep 'a' is not done, so a (no deps) wins");
}

#[test]
fn next_eligible_blocked_by_in_progress() {
    let plan =
        plan_with_changes(vec![change("a", Status::InProgress), change("b", Status::Pending)]);
    assert!(plan.next_eligible().is_none(), "an in-progress entry must block any new selection");
}

#[test]
fn next_eligible_none_when_finished() {
    // Post-2.0 the only terminal per-entry state is `Done`. A
    // plan whose entries are all `Done` is drained — `next_eligible`
    // must report nothing.
    let plan = plan_with_changes(vec![
        change("a", Status::Done),
        change("b", Status::Done),
        change("c", Status::Done),
    ]);
    assert!(plan.next_eligible().is_none());
}

#[test]
fn next_eligible_tiebreak() {
    let plan =
        plan_with_changes(vec![change("alpha", Status::Pending), change("beta", Status::Pending)]);
    let eligible = plan.next_eligible().expect("alpha should be first");
    assert_eq!(eligible.name, "alpha", "list-order tie-break must pick the first entry");
}

/// Drive `next_eligible` forward across the reference example plan,
/// marking each returned entry `done`, and assert the exact
/// traversal sequence.
#[test]
fn next_eligible_plan_forward() {
    let mut plan: Plan = serde_saphyr::from_str(PLAN_EXAMPLE_YAML).expect("parse plan fixture");
    for entry in &mut plan.entries {
        entry.status = Status::Pending;
    }

    let mut traversal = Vec::new();
    while let Some(next) = plan.next_eligible() {
        let name = next.name.clone();
        traversal.push(name.clone());
        let entry = plan
            .entries
            .iter_mut()
            .find(|c| c.name == name)
            .expect("returned name must exist in plan");
        entry.status = Status::Done;
    }

    let expected = [
        "user-registration",
        "email-verification",
        "registration-duplicate-email-crash",
        "notification-preferences",
        "extract-shared-validation",
        "product-catalog",
        "shopping-cart",
        "checkout-api",
        "checkout-ui",
    ];
    assert_eq!(
        traversal, expected,
        "next_eligible traversal should follow the §The Plan reference forward order"
    );
}

#[test]
fn next_eligible_blocks_mid_cycle() {
    let plan = plan_with_changes(vec![
        change("in-flight", Status::InProgress),
        change_with_deps("a", Status::Pending, &["b"]),
        change_with_deps("b", Status::Pending, &["a"]),
    ]);
    assert!(
        plan.next_eligible().is_none(),
        "in-progress entry must block selection before any dependency walk"
    );
}

#[test]
fn advance_next_reuses_in_progress() {
    let mut plan = plan_with_changes(vec![
        change("a", Status::Pending),
        change_with_deps("b", Status::Pending, &["a"]),
    ]);
    // First call transitions `a` to in-progress and returns it.
    let active = plan.advance_next().expect("advance ok");
    assert_eq!(active.unwrap().name, "a");
    assert_eq!(plan.entries[0].status, Status::InProgress);
    // Subsequent calls report the existing active entry without
    // moving any other entry.
    let again = plan.advance_next().expect("advance ok");
    assert_eq!(again.unwrap().name, "a", "active entry must be returned, not advanced past");
    assert_eq!(plan.entries[0].status, Status::InProgress);
    assert_eq!(plan.entries[1].status, Status::Pending);
}

#[test]
fn advance_reports_drained() {
    let mut plan = plan_with_changes(vec![change("a", Status::Done), change("b", Status::Done)]);
    let next = plan.advance_next().expect("advance ok");
    assert!(next.is_none(), "drained plan must report None");
    assert!(plan.is_drained());
}
