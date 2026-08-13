//! Per-target accepted-CID projection from `target.merge.wave-committed`.

use jiff::Timestamp;
use project::config::Layout;
use project::journal::{Event, EventKind, FactEpochRef, append_one, read_union};
use project::name::SliceName;
use project::snapshot::SnapshotId;
use project::wave::{EpochRef, Wave, accepted_cid};

const fn layout(root: &std::path::Path) -> Layout<'_> {
    Layout::new(root)
}

fn ts(second: i64) -> Timestamp {
    Timestamp::from_second(1_700_000_000 + second).expect("valid timestamp")
}

fn cid(ch: char) -> SnapshotId {
    SnapshotId::from_digest(&ch.to_string().repeat(64))
}

fn sample(base: SnapshotId) -> Wave {
    Wave::one_member(
        "demo",
        base,
        SliceName::from("login-flow"),
        cid('b'),
        vec![],
        EpochRef {
            writer: "local".into(),
            sequence: 1,
        },
    )
}

fn commit(layout: Layout<'_>, digest: &str, base: SnapshotId, result: SnapshotId, seq: i64) {
    append_one(
        layout,
        &Event::new(
            ts(seq),
            EventKind::TargetMergeWaveCommitted {
                target: "demo".into(),
                digest: digest.into(),
                members: vec!["login-flow".into()],
                base,
                result,
                commit_authorization: FactEpochRef {
                    writer: "local".into(),
                    sequence: 1,
                },
                identity_maps: vec![],
                baseline: None,
                deferred: vec![],
            },
        ),
    )
    .expect("append commit");
}

#[test]
fn none_before_any_wave() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let layout = layout(tmp.path());
    let events = read_union(layout).expect("union");
    assert_eq!(accepted_cid(layout, &events, "demo").expect("project"), None);
}

#[test]
fn first_wave_base() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let layout = layout(tmp.path());
    let wave = sample(cid('a'));
    wave.open(layout, ts(0)).expect("open");
    let events = read_union(layout).expect("union");
    assert_eq!(accepted_cid(layout, &events, "demo").expect("project"), Some(cid('a')));
}

#[test]
fn walks_commit_chain() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let layout = layout(tmp.path());
    let first = sample(cid('a'));
    let opened = first.open(layout, ts(0)).expect("open");
    commit(layout, opened.digest.as_str(), cid('a'), cid('c'), 1);

    let events = read_union(layout).expect("union");
    assert_eq!(accepted_cid(layout, &events, "demo").expect("project"), Some(cid('c')));

    commit(layout, "sha256:second", cid('c'), cid('d'), 2);
    let events = read_union(layout).expect("union");
    assert_eq!(accepted_cid(layout, &events, "demo").expect("project"), Some(cid('d')));
}

#[test]
fn broken_chain_is_typed() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let layout = layout(tmp.path());
    let wave = sample(cid('a'));
    let opened = wave.open(layout, ts(0)).expect("open");
    commit(layout, opened.digest.as_str(), cid('a'), cid('c'), 1);
    commit(layout, "sha256:broken", cid('e'), cid('f'), 2);

    let events = read_union(layout).expect("union");
    let err = accepted_cid(layout, &events, "demo").expect_err("broken chain");
    assert!(err.to_string().contains("target-accepted-cid-broken-chain"), "{err}");
}
