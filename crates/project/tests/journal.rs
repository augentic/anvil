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
use project::snapshot::SnapshotId;

fn cid(ch: char) -> SnapshotId {
    SnapshotId::from_digest(&ch.to_string().repeat(64))
}

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
fn reads_prior_actor_wire() {
    // Pre-rename journals and epoch refs used `actor`; read_union skips
    // unparseable lines, so missing aliases would silently drop history.
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let root = tmp.path();
    let events_dir = root.join(".emery/change/events");
    std::fs::create_dir_all(&events_dir).expect("mkdir");
    let claimed = concat!(
        r#"{"timestamp":"2023-11-14T22:13:20Z","actor":"alice","sequence":1,"#,
        r#""event":"slice.claimed","payload":{"slice-name":"orders-api"}}"#,
        "\n",
    );
    std::fs::write(events_dir.join("alice.jsonl"), claimed).expect("write prior journal");

    let events = read_union(layout(root)).expect("union");
    assert_eq!(events.len(), 1, "prior actor envelopes must stay in the union");
    assert_eq!(events[0].writer, "alice");
    assert_eq!(events[0].sequence, 1);
    assert!(matches!(
        &events[0].kind,
        EventKind::SliceClaimed { slice_name } if slice_name.as_str() == "orders-api"
    ));
    assert_eq!(
        claim::project(&events).owner(&"orders-api".into()),
        Some("alice"),
        "claim via prior actor payload projects ownership"
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
#[expect(
    unsafe_code,
    reason = "EMERY_WRITER is the journal writer seam; nextest isolates the process"
)]
fn writer_id_env() {
    // RFC-96 D4: a non-empty EMERY_WRITER wins on every deployment;
    // whitespace and empty values fall back to the local default.
    // SAFETY: nextest isolates the test process; no concurrent reader.
    unsafe { std::env::remove_var("EMERY_WRITER") };
    assert_eq!(project::journal::writer_id(), DEFAULT_WRITER);
    // SAFETY: as above.
    unsafe { std::env::set_var("EMERY_WRITER", "  ") };
    assert_eq!(project::journal::writer_id(), DEFAULT_WRITER, "whitespace falls back");
    // SAFETY: as above.
    unsafe { std::env::set_var("EMERY_WRITER", "") };
    assert_eq!(project::journal::writer_id(), DEFAULT_WRITER, "empty falls back");
    // SAFETY: as above.
    unsafe { std::env::set_var("EMERY_WRITER", " alice ") };
    assert_eq!(project::journal::writer_id(), "alice", "trimmed value wins");
    // SAFETY: as above.
    unsafe { std::env::remove_var("EMERY_WRITER") };
}

#[test]
fn deferral_round_trip() {
    // RFC-86a D2: dotted-kebab wire id and kebab-case payload keys on
    // the deferral fact, stable through serde and the file union.
    let deferred = Event {
        timestamp: ts(0),
        writer: "alice".into(),
        sequence: 1,
        kind: EventKind::GapDeferred {
            slice: "auth-login".into(),
            req: "REQ-003".into(),
            requirement_digest: "sha256:abc123".into(),
            reason: "reset path deferred to next change".into(),
        },
    };
    let wire = serde_json::to_string(&deferred).expect("serialize");
    assert!(wire.contains(r#""event":"gap.deferred""#), "{wire}");
    assert!(wire.contains(r#""slice":"auth-login""#), "{wire}");
    assert!(wire.contains(r#""requirement-digest":"sha256:abc123""#), "{wire}");
    assert_eq!(serde_json::from_str::<Event>(&wire).expect("parse"), deferred);

    // Through the per-writer file and back into the union.
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let layout = layout(tmp.path());
    append_for(layout, "alice", std::slice::from_ref(&deferred)).expect("append deferred");
    let events = read_union(layout).expect("union");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, deferred.kind);
}

#[test]
fn wave_defer_roundtrip() {
    // RFC-86a D5: the wave-commit fact snapshots the deferred member
    // set it carried; a debt-free fact omits the field and prior
    // journals without it stay parseable.
    let committed = Event {
        timestamp: ts(0),
        writer: "alice".into(),
        sequence: 1,
        kind: EventKind::TargetMergeWaveCommitted {
            target: "mock".into(),
            digest: "sha256:abc".into(),
            members: vec!["auth-login".into()],
            base: cid('a'),
            result: cid('b'),
            commit_authorization: FactEpochRef {
                writer: "alice".into(),
                sequence: 1,
            },
            identity_maps: vec![],
            baseline: None,
            deferred: vec![project::journal::DeferredMember {
                req: "REQ-007".into(),
                status: artifacts::spec::provenance::RequirementStatus::Conflict,
                requirement_digest: "sha256:def".into(),
            }],
        },
    };
    let wire = serde_json::to_string(&committed).expect("serialize");
    assert!(wire.contains(r#""deferred":[{"#), "{wire}");
    assert!(wire.contains(r#""req":"REQ-007""#), "{wire}");
    assert!(wire.contains(r#""status":"conflict""#), "{wire}");
    assert!(wire.contains(r#""requirement-digest":"sha256:def""#), "{wire}");
    assert_eq!(serde_json::from_str::<Event>(&wire).expect("parse"), committed);

    // Empty set: skipped on the wire, defaulted on read.
    let clean = Event {
        timestamp: ts(1),
        writer: "alice".into(),
        sequence: 2,
        kind: EventKind::TargetMergeWaveCommitted {
            target: "mock".into(),
            digest: "sha256:abc".into(),
            members: vec!["auth-login".into()],
            base: cid('a'),
            result: cid('b'),
            commit_authorization: FactEpochRef {
                writer: "alice".into(),
                sequence: 1,
            },
            identity_maps: vec![],
            baseline: None,
            deferred: vec![],
        },
    };
    let wire = serde_json::to_string(&clean).expect("serialize");
    assert!(!wire.contains("deferred"), "empty snapshot stays off the wire: {wire}");
    assert_eq!(serde_json::from_str::<Event>(&wire).expect("parse"), clean);
}

#[test]
fn append_stamps_writer() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let root = tmp.path();
    let layout = layout(root);

    append_for(layout, "operator-a", &[build_started(0, "alpha"), build_started(1, "beta")])
        .expect("append batch");
    append_for(layout, "operator-a", &[build_started(2, "gamma")]).expect("append one more");

    let path = root.join(".emery/change/events/operator-a.jsonl");
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
fn append_skips_corrupt_tail() {
    // A corrupt trailing line must not reset the sequence to 1; the
    // append walks back to the newest parseable event. The union read
    // stays strict: authority readers get a typed error until the
    // corrupt line is repaired (S13 / CC-11).
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let root = tmp.path();
    let layout = layout(root);

    append_for(layout, "operator-a", &[build_started(0, "alpha"), build_started(1, "beta")])
        .expect("append batch");
    let path = root.join(".emery/change/events/operator-a.jsonl");
    let mut contents = std::fs::read_to_string(&path).expect("writer log");
    contents.push_str("{not json\n");
    std::fs::write(&path, contents).expect("corrupt trailing line");

    append_for(layout, "operator-a", &[build_started(2, "gamma")]).expect("append after corrupt");

    let sequences: Vec<u64> = std::fs::read_to_string(&path)
        .expect("writer log")
        .lines()
        .filter_map(|line| serde_json::from_str::<Event>(line).ok())
        .map(|event| event.sequence)
        .collect();
    assert_eq!(sequences, vec![1, 2, 3], "sequence continues past the corrupt line");

    let err = read_union(layout).expect_err("strict union refuses the corrupt line");
    match err {
        error::Error::Diag { code, .. } => assert_eq!(code, "journal-line-malformed"),
        other => panic!("expected Diag, got {other:?}"),
    }
}

#[test]
fn events_dir_unreadable_errs() {
    // S13 / CC-11: an unreadable events directory is an I/O error on
    // the authority read — never an empty union.
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let root = tmp.path();
    std::fs::create_dir_all(root.join(".emery/change")).expect("change home");
    std::fs::write(root.join(".emery/change/events"), "not a directory").expect("blocker");
    let err = read_union(layout(root)).expect_err("unreadable events dir");
    assert!(matches!(err, error::Error::Io(_)), "expected Io, got {err:?}");
}

#[tokio::test]
async fn show_stays_lenient() {
    // `journal show` is the one lenient reader: the observability
    // projection still lists the parseable events beside a corrupt
    // line that fails every authority read.
    let project = Session::scripted("demo", Vec::new());
    let root = project.root();
    let layout = layout(root);
    append_for(layout, "alpha", &[build_started(0, "a1")]).expect("append");
    let path = root.join(".emery/change/events/alpha.jsonl");
    let mut contents = std::fs::read_to_string(&path).expect("writer log");
    contents.push_str("{not json\n");
    std::fs::write(&path, contents).expect("corrupt line");

    read_union(layout).expect_err("authority read refuses");
    let body = run::<handlers::Show, _, _>(
        project.provider(),
        handlers::ShowInput {
            filter: None,
            limit: None,
        },
    )
    .await
    .expect("show stays lenient");
    assert_eq!(body.count, 1);
}

#[test]
fn corrupt_log_refused() {
    // A non-empty log with zero parseable events fails closed rather
    // than silently restarting at sequence 1.
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let root = tmp.path();
    let layout = layout(root);
    let events_dir = root.join(".emery/change/events");
    std::fs::create_dir_all(&events_dir).expect("mkdir");
    std::fs::write(events_dir.join("operator-a.jsonl"), "{not json\ngarbage\n")
        .expect("corrupt log");

    let err = append_for(layout, "operator-a", &[build_started(0, "alpha")])
        .expect_err("corrupt log refused");
    match err {
        error::Error::Diag { code, .. } => assert_eq!(code, "journal-log-corrupt"),
        other => panic!("expected Diag, got {other:?}"),
    }
}

#[test]
fn union_orders_timestamp() {
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
fn append_uses_default() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let root = tmp.path();
    let layout = layout(root);

    append_one(layout, &build_started(0, "solo")).expect("append");

    let writer_path = root.join(".emery/change/events").join(format!("{DEFAULT_WRITER}.jsonl"));
    assert!(writer_path.is_file(), "default writer file");
    assert!(!root.join(".emery/journal.jsonl").exists(), "legacy journal.jsonl is not written");

    let events = read_union(layout).expect("union");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].writer, DEFAULT_WRITER);
    assert_eq!(events[0].sequence, 1);
}

#[test]
fn emit_best_effort_writes() {
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
fn append_rejects_path() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let err = append_for(layout(tmp.path()), "evil/name", &[build_started(0, "x")])
        .expect_err("path separator refused");
    match err {
        error::Error::Diag { code, .. } => assert_eq!(code, "journal-writer-invalid"),
        other => panic!("expected Diag, got {other:?}"),
    }
}

#[test]
fn missing_events_dir_unions() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let events = read_union(layout(tmp.path())).expect("missing is empty");
    assert!(events.is_empty());
}

#[tokio::test]
async fn show_merges_per_writer() {
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

#[test]
fn concurrent_claims() {
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
fn slice_second_writer() {
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
fn release_clears_live_claim() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let layout = layout(tmp.path());

    append_for(layout, "alice", &[claimed(1, "orders-api")]).expect("claim");
    append_for(layout, "alice", &[released(2, "orders-api")]).expect("release");
    let ownership = claim::project(&read_union(layout).expect("union"));
    assert!(ownership.is_empty(), "release by owner clears the claim");

    append_for(layout, "bob", &[claimed(3, "orders-api")]).expect("bob after release");
    let ownership = claim::project(&read_union(layout).expect("union"));
    assert_eq!(ownership.owner(&"orders-api".into()), Some("bob"), "slice free after release");
}

#[test]
fn append_refuses_review() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let layout = layout(tmp.path());
    let event = Event::new(
        ts(0),
        EventKind::SystemWaveReviewed {
            wave: "deliver".into(),
            handoff_digest: cid('a'),
        },
    );
    let err = append_for(layout, "local", &[event]).expect_err("refused");
    assert!(err.to_string().contains("journal-event-read-only"), "{err}");
}

#[test]
fn review_event_round_trip() {
    let event = Event {
        timestamp: ts(0),
        writer: "local".into(),
        sequence: 1,
        kind: EventKind::SystemWaveReviewed {
            wave: "deliver".into(),
            handoff_digest: cid('a'),
        },
    };
    let wire = serde_json::to_string(&event).expect("serialize");
    assert!(wire.contains(r#""event":"system.wave.reviewed""#), "{wire}");
    assert!(wire.contains(r#""handoff-digest""#), "{wire}");
    assert_eq!(serde_json::from_str::<Event>(&wire).expect("parse"), event);
    let digest = event.digest().expect("digest");
    SnapshotId::parse(digest.as_str()).expect("sha256");
}
