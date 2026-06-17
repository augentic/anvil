use std::collections::BTreeMap;

use super::super::model::{Lifecycle, Plan, Status};
use super::super::test_fixtures::{change, change_with_deps};

#[test]
fn remove_drops_pending_entry() {
    let mut plan = Plan {
        name: "p".into(),
        lifecycle: Lifecycle::Pending,
        sources: BTreeMap::default(),
        entries: vec![change("a", Status::Pending), change("b", Status::Pending)],
    };
    plan.remove("a").unwrap();
    assert_eq!(plan.entries.len(), 1);
    assert_eq!(plan.entries[0].name, "b");
}

#[test]
fn remove_refuses_when_not_replaceable() {
    let mut plan = Plan {
        name: "p".into(),
        lifecycle: Lifecycle::Approved,
        sources: BTreeMap::default(),
        entries: vec![change("a", Status::Pending)],
    };
    let err = plan.remove("a").unwrap_err();
    match err {
        specify_error::Error::Validation { code, .. } => {
            assert_eq!(code, "plan-remove-plan-not-replaceable");
        }
        other => panic!("expected Validation, got {other:?}"),
    }
}

#[test]
fn remove_refuses_when_depended_on() {
    let mut plan = Plan {
        name: "p".into(),
        lifecycle: Lifecycle::Pending,
        sources: BTreeMap::default(),
        entries: vec![change("a", Status::Pending), change_with_deps("b", Status::Pending, &["a"])],
    };
    let err = plan.remove("a").unwrap_err();
    match err {
        specify_error::Error::Validation { code, .. } => {
            assert_eq!(code, "plan-remove-entry-referenced");
        }
        other => panic!("expected Validation, got {other:?}"),
    }
}
