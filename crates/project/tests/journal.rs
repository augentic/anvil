//! Per-actor event log I/O (RFC-86 D3): append stamps actor/sequence,
//! each actor writes only their file, and readers union by
//! `(timestamp, actor, sequence)`.

use jiff::Timestamp;
use project::config::Layout;
use project::journal::{
    DEFAULT_ACTOR, Event, EventKind, append_for, append_one, read_union,
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
    append_for(layout, "alpha", &[build_started(5, "a1"), build_started(5, "a2")])
        .expect("alpha");
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
        vec![
            ("charlie", 1, "c1"),
            ("alpha", 1, "a1"),
            ("alpha", 2, "a2"),
            ("bravo", 1, "b1"),
        ]
    );
}

#[test]
fn append_one_uses_default_actor_and_dual_writes_legacy_journal() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let root = tmp.path();
    let layout = layout(root);

    append_one(layout, &build_started(0, "solo")).expect("append");

    let actor_path = root.join(".emery/events").join(format!("{DEFAULT_ACTOR}.jsonl"));
    assert!(actor_path.is_file(), "default actor file");
    let legacy = root.join(".emery/journal.jsonl");
    assert!(legacy.is_file(), "legacy dual-write bridge");

    let events = read_union(layout).expect("union");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].actor, DEFAULT_ACTOR);
    assert_eq!(events[0].sequence, 1);

    let legacy_line = std::fs::read_to_string(&legacy).expect("legacy");
    let parsed: Event = serde_json::from_str(legacy_line.trim()).expect("legacy parses");
    assert_eq!(parsed, events[0]);
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
