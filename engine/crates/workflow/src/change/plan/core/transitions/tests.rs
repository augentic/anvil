use super::super::{change, plan_with_changes};
use super::*;

#[test]
fn transition_in_progress_to_done() {
    let mut plan =
        plan_with_changes(vec![change("a", Status::Pending), change("b", Status::InProgress)]);
    plan.transition("b", Status::Done).expect("in-progress -> done ok");
    assert_eq!(plan.entries.iter().find(|c| c.name == "b").unwrap().status, Status::Done);
    let Err(Error::Diag { code, detail }) = plan.transition("a", Status::Done) else {
        panic!("Pending -> Done must Err with plan-transition diag");
    };
    assert_eq!(code, "plan-transition");
    assert!(detail.contains("Pending") && detail.contains("Done"), "endpoints in: {detail:?}");
    assert_eq!(plan.entries[0].status, Status::Pending, "status not mutated on illegal edge");
}

#[test]
fn pending_to_approved_then_terminal() {
    let mut plan = plan_with_changes(vec![change("a", Status::Pending)]);
    plan.transition_lifecycle(Lifecycle::Approved).expect("pending -> approved ok");
    assert_eq!(plan.lifecycle, Lifecycle::Approved);
    let Err(Error::Diag { code, detail }) = plan.transition_lifecycle(Lifecycle::Approved) else {
        panic!("approved -> approved must Err");
    };
    assert_eq!(code, "plan-lifecycle-transition");
    assert!(detail.contains("Approved"), "endpoint in: {detail:?}");
}

#[test]
fn undo_walks_status_one_rung() {
    let mut plan = plan_with_changes(vec![change("slice", Status::Done)]);
    let (from, to) = plan.transition_undo("slice").expect("done -> in-progress ok");
    assert_eq!((from, to), (Status::Done, Status::InProgress));
    assert_eq!(plan.entries[0].status, Status::InProgress);

    let (from, to) = plan.transition_undo("slice").expect("in-progress -> pending ok");
    assert_eq!((from, to), (Status::InProgress, Status::Pending));
    assert_eq!(plan.entries[0].status, Status::Pending);
}

#[test]
fn undo_refuses_from_pending() {
    let mut plan = plan_with_changes(vec![change("slice", Status::Pending)]);
    let Err(Error::Diag { code, detail }) = plan.transition_undo("slice") else {
        panic!("undo from pending must Err with plan-transition-undo diag");
    };
    assert_eq!(code, "plan-transition-undo");
    assert!(detail.contains("pending"), "endpoint in: {detail:?}");
    assert_eq!(plan.entries[0].status, Status::Pending, "status not mutated on illegal undo");
}

#[test]
fn undo_unknown_entry_diag() {
    let mut plan = plan_with_changes(vec![change("known", Status::InProgress)]);
    let Err(Error::Diag { code, .. }) = plan.transition_undo("ghost") else {
        panic!("unknown entry must Err with plan-entry-not-found");
    };
    assert_eq!(code, "plan-entry-not-found");
}

#[test]
fn init_then_approved_auto_approve() {
    // auto-approve Gate-1 contract: `--auto-review` composes `Plan::init` with
    // `Plan::transition_lifecycle(Reviewed)` before the single
    // atomic save. The resulting in-memory plan must carry
    // `lifecycle: approved` so the post-init `Plan::save` writes
    // `lifecycle: approved` directly with no transient `pending`
    // round trip through disk.
    let mut plan = Plan::init("fresh", std::collections::BTreeMap::new()).expect("init fresh ok");
    assert_eq!(plan.lifecycle, Lifecycle::Pending, "fresh init defaults to pending");
    plan.transition_lifecycle(Lifecycle::Approved)
        .expect("--auto-review composes init + lifecycle stamp");
    assert_eq!(
        plan.lifecycle,
        Lifecycle::Approved,
        "in-memory plan must carry approved before save under --auto-review"
    );
}
