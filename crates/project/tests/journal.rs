//! Per-actor event log I/O (RFC-86 D3): append stamps actor/sequence,
//! each actor writes only their file, and readers union by
//! `(timestamp, actor, sequence)`. `journal show` merges that union.
//! Exclusive per-slice claims (RFC-86 D7 / D23) project from the union.

use jiff::Timestamp;
use mock::invoke::run;
use mock::session::Session;
use project::config::Layout;
use project::journal::{
    DEFAULT_ACTOR, Event, EventKind, append_for, append_one, claim, emit_best_effort, handlers,
    read_union,
};

const fn layout(root: &std::path::Path) -> Layout<'_> {
    Layout::new(root)
}

fn ts(second: i64) -> Timestamp {
    Timestamp::from_second(1_700_000_000 + second).expect("valid timestamp")
}

fn build_started(second: i64, slice: &str) -> Event {
    Event::new(
        ts(second),
        EventKind::SliceBuildStarted {
            slice_name: slice.into(),
        },
    )
}

#[test]
fn append_stamps_actor_and_monotonic_sequence() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let root = tmp.path();
    let layout = layout(root);

    append_for(layout, "operator-a", &[build_started(0, "alpha"), build_started(1, "beta")])
        .expect("append batch");
    append_for(layout, "operator-a", &[build_started(2, "gamma")]).expect("append one more");

    let path = root.join(".emery/events/operator-a.jsonl");
    assert!(path.is_file(), "actor file created");
    assert!(!root.join(".emery/journal.jsonl").exists(), "single-file journal is not written");
    let events = read_union(layout).expect("union");
    assert_eq!(events.len(), 3);
    assert!(events.iter().all(|event| event.actor == "operator-a"));
    assert_eq!(
        events.iter().map(|event| event.sequence).collect::<Vec<_>>(),
        vec![1, 2, 3],
        "sequence is 1-based and continues across appends"
    );
}

#[test]
fn union_orders_by_timestamp_actor_sequence() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let root = tmp.path();
    let layout = layout(root);

    // Same timestamp, different actors — actor name breaks ties.
    append_for(layout, "bravo", &[build_started(5, "b1")]).expect("bravo");
    append_for(layout, "alpha", &[build_started(5, "a1"), build_started(5, "a2")]).expect("alpha");
    // Earlier timestamp sorts first regardless of append order.
    append_for(layout, "charlie", &[build_started(1, "c1")]).expect("charlie");

    let events = read_union(layout).expect("union");
    let keys: Vec<(&str, u64, &str)> = events
        .iter()
        .map(|event| {
            let slice = match &event.kind {
                EventKind::SliceBuildStarted { slice_name } => slice_name.as_str(),
                other => panic!("unexpected kind: {other:?}"),
            };
            (event.actor.as_str(), event.sequence, slice)
        })
        .collect();
    assert_eq!(
        keys,
        vec![("charlie", 1, "c1"), ("alpha", 1, "a1"), ("alpha", 2, "a2"), ("bravo", 1, "b1"),]
    );
}

#[test]
fn append_one_uses_default_actor_only() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let root = tmp.path();
    let layout = layout(root);

    append_one(layout, &build_started(0, "solo")).expect("append");

    let actor_path = root.join(".emery/events").join(format!("{DEFAULT_ACTOR}.jsonl"));
    assert!(actor_path.is_file(), "default actor file");
    assert!(!root.join(".emery/journal.jsonl").exists(), "legacy journal.jsonl is not written");

    let events = read_union(layout).expect("union");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].actor, DEFAULT_ACTOR);
    assert_eq!(events[0].sequence, 1);
}

#[test]
fn emit_best_effort_writes_per_actor_log() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let root = tmp.path();
    let layout = layout(root);

    emit_best_effort(
        layout,
        ts(0),
        EventKind::SliceBuildStarted {
            slice_name: "solo".into(),
        },
        "test-emit",
    );

    let events = read_union(layout).expect("union");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].actor, DEFAULT_ACTOR);
    assert!(
        !root.join(".emery/journal.jsonl").exists(),
        "emit must not dual-write the legacy file"
    );
}

#[test]
fn append_for_rejects_path_separator_actor() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let err = append_for(layout(tmp.path()), "evil/name", &[build_started(0, "x")])
        .expect_err("path separator refused");
    match err {
        error::Error::Diag { code, .. } => assert_eq!(code, "journal-actor-invalid"),
        other => panic!("expected Diag, got {other:?}"),
    }
}

#[test]
fn missing_events_dir_unions_empty() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let events = read_union(layout(tmp.path())).expect("missing is empty");
    assert!(events.is_empty());
}

#[tokio::test]
async fn show_merges_per_actor_union() {
    let project = Session::scripted("demo", Vec::new());
    let root = project.root();
    let layout = layout(root);

    append_for(layout, "bravo", &[build_started(5, "b1")]).expect("bravo");
    append_for(layout, "alpha", &[build_started(1, "a1")]).expect("alpha");

    let body = run::<handlers::Show, _, _>(
        project.provider(),
        handlers::ShowInput {
            filter: Some("slice.build".into()),
            limit: None,
        },
    )
    .await
    .expect("show");
    assert_eq!(body.count, 2);
    assert_eq!(body.events[0].actor, "alpha");
    assert_eq!(body.events[1].actor, "bravo");

    let limited = run::<handlers::Show, _, _>(
        project.provider(),
        handlers::ShowInput {
            filter: Some("slice.build".into()),
            limit: Some(1),
        },
    )
    .await
    .expect("show limit");
    assert_eq!(limited.count, 1);
    assert_eq!(limited.events[0].actor, "bravo", "limit keeps the newest match");
}

fn claimed(second: i64, slice: &str) -> Event {
    Event::new(
        ts(second),
        EventKind::SliceClaimed {
            slice_name: slice.into(),
        },
    )
}

fn released(second: i64, slice: &str) -> Event {
    Event::new(
        ts(second),
        EventKind::SliceReleased {
            slice_name: slice.into(),
        },
    )
}

fn retracted(second: i64, actor: &str, sequence: u64) -> Event {
    Event::new(
        ts(second),
        EventKind::FactRetracted {
            actor: actor.into(),
            sequence,
        },
    )
}

#[test]
fn concurrent_claims_on_different_slices() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let layout = layout(tmp.path());

    append_for(layout, "alice", &[claimed(1, "orders-api")]).expect("alice claim");
    append_for(layout, "bob", &[claimed(1, "orders-ui")]).expect("bob claim");

    let ownership = claim::project(&read_union(layout).expect("union"));
    assert_eq!(ownership.len(), 2);
    assert_eq!(ownership.owner(&"orders-api".into()), Some("alice"));
    assert_eq!(ownership.owner(&"orders-ui".into()), Some("bob"));

    claim::ensure_claimable(&ownership, &"orders-api".into(), "alice").expect("idempotent");
    claim::ensure_claimable(&ownership, &"checkout".into(), "alice")
        .expect("third slice still free");
}

#[test]
fn same_slice_second_actor_conflicts() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let layout = layout(tmp.path());

    append_for(layout, "alice", &[claimed(1, "orders-api")]).expect("alice claim");
    let ownership = claim::project(&read_union(layout).expect("union"));

    let err = claim::claim(&ownership, "orders-api".into(), "bob").expect_err("conflict");
    match err {
        error::Error::Validation { code, detail } => {
            assert_eq!(code, "slice-claim-conflict");
            assert!(detail.contains("alice"), "{detail}");
            assert!(detail.contains("bob"), "{detail}");
        }
        other => panic!("expected Validation, got {other:?}"),
    }

    // A conflicting append must not be written — fixtures union after
    // a refused claim still show one owner.
    assert_eq!(
        claim::project(&read_union(layout).expect("union")).owner(&"orders-api".into()),
        Some("alice")
    );
}

#[test]
fn release_and_retract_clear_live_claim() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let layout = layout(tmp.path());

    append_for(layout, "alice", &[claimed(1, "orders-api")]).expect("claim");
    append_for(layout, "alice", &[released(2, "orders-api")]).expect("release");
    assert!(
        claim::project(&read_union(layout).expect("union")).is_empty(),
        "release by owner clears the claim"
    );

    append_for(layout, "bob", &[claimed(3, "orders-api")]).expect("bob after release");
    // Retract bob's claim (sequence 1 in bob's file).
    append_for(layout, "bob", &[retracted(4, "bob", 1)]).expect("retract");
    let ownership = claim::project(&read_union(layout).expect("union"));
    assert!(ownership.is_empty(), "retracted claim is absent from projection");
    claim::ensure_claimable(&ownership, &"orders-api".into(), "alice")
        .expect("slice free after retract");
}

#[test]
fn retract_of_retract_restores_claim() {
    // claim (seq 1) → retract claim (seq 2) → retract the retract (seq 3)
    // restores the original claim.
    let events = vec![
        Event {
            timestamp: ts(1),
            actor: "alice".into(),
            sequence: 1,
            kind: EventKind::SliceClaimed {
                slice_name: "orders-api".into(),
            },
        },
        Event {
            timestamp: ts(2),
            actor: "alice".into(),
            sequence: 2,
            kind: EventKind::FactRetracted {
                actor: "alice".into(),
                sequence: 1,
            },
        },
        Event {
            timestamp: ts(3),
            actor: "alice".into(),
            sequence: 3,
            kind: EventKind::FactRetracted {
                actor: "alice".into(),
                sequence: 2,
            },
        },
    ];
    let ownership = claim::project(&events);
    assert_eq!(ownership.owner(&"orders-api".into()), Some("alice"));
}
