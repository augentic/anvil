//! Per-writer event log I/O (RFC-86 D3): append stamps writer/sequence,
//! each writer writes only its file, and readers union by
//! `(timestamp, writer, sequence)`. `journal show` merges that union.
//! Exclusive per-slice claims (RFC-86 D7 / D23) project from the union.

use jiff::Timestamp;
use mock::invoke::run;
use mock::session::Session;
use project::config::Layout;
use project::journal::{
    DEFAULT_WRITER, Event, EventKind, FactEpochRef, append_for, append_one, claim,
    emit_best_effort, handlers, read_union,
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
fn reads_prior_actor_wire_fields() {
    // Pre-rename journals and epoch refs used `actor`; read_union skips
    // unparseable lines, so missing aliases would silently drop history.
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let root = tmp.path();
    let events_dir = root.join(".emery/events");
    std::fs::create_dir_all(&events_dir).expect("mkdir");
    let claimed = concat!(
        r#"{"timestamp":"2023-11-14T22:13:20Z","actor":"alice","sequence":1,"#,
        r#""event":"slice.claimed","payload":{"slice-name":"orders-api"}}"#,
        "\n",
    );
    let retracted = concat!(
        r#"{"timestamp":"2023-11-14T22:13:21Z","actor":"alice","sequence":2,"#,
        r#""event":"fact.retracted","payload":{"actor":"alice","sequence":1}}"#,
        "\n",
    );
    std::fs::write(events_dir.join("alice.jsonl"), format!("{claimed}{retracted}"))
        .expect("write prior journal");

    let events = read_union(layout(root)).expect("union");
    assert_eq!(events.len(), 2, "prior actor envelopes must stay in the union");
    assert_eq!(events[0].writer, "alice");
    assert_eq!(events[0].sequence, 1);
    assert!(matches!(
        &events[0].kind,
        EventKind::SliceClaimed { slice_name } if slice_name.as_str() == "orders-api"
    ));
    assert!(matches!(
        &events[1].kind,
        EventKind::FactRetracted { writer, sequence: 1 } if writer == "alice"
    ));
    assert!(
        claim::project(&events).is_empty(),
        "retracted claim via prior actor payload stays retracted"
    );

    let epoch: FactEpochRef =
        serde_json::from_str(r#"{"actor":"local","sequence":7}"#).expect("prior epoch ref");
    assert_eq!(epoch.writer, "local");
    assert_eq!(epoch.sequence, 7);

    let stamped = Event {
        timestamp: ts(0),
        writer: "bob".into(),
        sequence: 1,
        kind: EventKind::SliceClaimed {
            slice_name: "orders-ui".into(),
        },
    };
    let wire = serde_json::to_string(&stamped).expect("serialize");
    assert!(wire.contains(r#""writer":"bob""#), "{wire}");
    assert!(!wire.contains(r#""actor""#), "new writes emit writer only: {wire}");
}

#[test]
fn append_stamps_writer_and_monotonic_sequence() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let root = tmp.path();
    let layout = layout(root);

    append_for(layout, "operator-a", &[build_started(0, "alpha"), build_started(1, "beta")])
        .expect("append batch");
    append_for(layout, "operator-a", &[build_started(2, "gamma")]).expect("append one more");

    let path = root.join(".emery/events/operator-a.jsonl");
    assert!(path.is_file(), "writer file created");
    assert!(!root.join(".emery/journal.jsonl").exists(), "single-file journal is not written");
    let events = read_union(layout).expect("union");
    assert_eq!(events.len(), 3);
    assert!(events.iter().all(|event| event.writer == "operator-a"));
    assert_eq!(
        events.iter().map(|event| event.sequence).collect::<Vec<_>>(),
        vec![1, 2, 3],
        "sequence is 1-based and continues across appends"
    );
}

#[test]
fn union_orders_by_timestamp_writer_sequence() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let root = tmp.path();
    let layout = layout(root);

    // Same timestamp, different writers — writer name breaks ties.
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
            (event.writer.as_str(), event.sequence, slice)
        })
        .collect();
    assert_eq!(
        keys,
        vec![("charlie", 1, "c1"), ("alpha", 1, "a1"), ("alpha", 2, "a2"), ("bravo", 1, "b1"),]
    );
}

#[test]
fn append_one_uses_default_writer_only() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let root = tmp.path();
    let layout = layout(root);

    append_one(layout, &build_started(0, "solo")).expect("append");

    let writer_path = root.join(".emery/events").join(format!("{DEFAULT_WRITER}.jsonl"));
    assert!(writer_path.is_file(), "default writer file");
    assert!(!root.join(".emery/journal.jsonl").exists(), "legacy journal.jsonl is not written");

    let events = read_union(layout).expect("union");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].writer, DEFAULT_WRITER);
    assert_eq!(events[0].sequence, 1);
}

#[test]
fn emit_best_effort_writes_per_writer_log() {
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
    assert_eq!(events[0].writer, DEFAULT_WRITER);
    assert!(
        !root.join(".emery/journal.jsonl").exists(),
        "emit must not dual-write the legacy file"
    );
}

#[test]
fn append_for_rejects_path_separator_writer() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let err = append_for(layout(tmp.path()), "evil/name", &[build_started(0, "x")])
        .expect_err("path separator refused");
    match err {
        error::Error::Diag { code, .. } => assert_eq!(code, "journal-writer-invalid"),
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
async fn show_merges_per_writer_union() {
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
    assert_eq!(body.events[0].writer, "alpha");
    assert_eq!(body.events[1].writer, "bravo");

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
    assert_eq!(limited.events[0].writer, "bravo", "limit keeps the newest match");
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

fn retracted(second: i64, writer: &str, sequence: u64) -> Event {
    Event::new(
        ts(second),
        EventKind::FactRetracted {
            writer: writer.into(),
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
fn same_slice_second_writer_conflicts() {
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
            writer: "alice".into(),
            sequence: 1,
            kind: EventKind::SliceClaimed {
                slice_name: "orders-api".into(),
            },
        },
        Event {
            timestamp: ts(2),
            writer: "alice".into(),
            sequence: 2,
            kind: EventKind::FactRetracted {
                writer: "alice".into(),
                sequence: 1,
            },
        },
        Event {
            timestamp: ts(3),
            writer: "alice".into(),
            sequence: 3,
            kind: EventKind::FactRetracted {
                writer: "alice".into(),
                sequence: 2,
            },
        },
    ];
    let ownership = claim::project(&events);
    assert_eq!(ownership.owner(&"orders-api".into()), Some("alice"));
}
