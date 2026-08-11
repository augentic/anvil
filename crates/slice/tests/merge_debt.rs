//! RFC-86a D5 — debt conservation at the merge fold: gap-status rows
//! without scenarios fold into the baseline with `Status:` preserved
//! (conflict arms intact, no downgrade) and final `REQ-NNN` assigned.
//! The note-stamping and wave-snapshot halves ride the plan-owned loop
//! and live in `crates/change/tests/merge_debt.rs`.

use std::fs;

use jiff::Timestamp;
use mock::session::Session;
use project::journal::{EventKind, read_union};
use project::name::SliceName;
use project::seam::Workspaces as _;
use project::snapshot::SnapshotId;
use project::wave::{EpochRef, Wave};

fn ts() -> Timestamp {
    Timestamp::from_second(1_700_000_000).expect("valid timestamp")
}

/// Standalone merge (no `plan.yaml`): the claim gate self-skips and no
/// deferral facts exist, so this exercises the fold and validation
/// alone.
async fn merge(
    session: &Session, slice: &str,
) -> Result<slice::orchestrate::MergeOutcome, error::Error> {
    let layout = project::config::Layout::new(session.root());
    slice::orchestrate::merge(session.provider(), layout, ts(), slice, false).await
}

/// The unknown row is a gap statement with no scenario; the conflict
/// row is only its two arms' `Note:` lines — both non-operative by
/// construction (RFC-86a D5).
const fn gap_delta() -> &'static str {
    "### Requirement: greeting error handling [unknown]\n\
     ID: REQ-001\n\
     Sources: []\n\
     Status: unknown\n\n\
     The greeting service handles errors; behaviour is not evidenced.\n\n\
     ### Requirement: session TTL [conflict]\n\
     ID: REQ-002\n\
     Sources: docs, code\n\
     Status: conflict\n\n\
     Note: docs says 30 minutes\n\
     Note: code says 15 minutes\n"
}

const fn gap_model() -> &'static str {
    r#"version: 1
slice: greeting
requirements:
  - id: REQ-001
    title: greeting error handling
    status: unknown
    domain: greeting
    sources: []
    claims: []
    statement: The greeting service handles errors; behaviour is not evidenced.
  - id: REQ-002
    title: session TTL
    status: conflict
    domain: greeting
    sources: [docs, code]
    claims: []
    statement: Session TTL is contested between docs and code.
    notes: "Note: docs says 30 minutes\nNote: code says 15 minutes"
tasks: []
"#
}

fn stage_built_gap_slice(session: &Session) {
    let root = session.root();
    let slice_dir = root.join(".emery/slices/greeting");
    let specs = slice_dir.join("specs/greeting");
    fs::create_dir_all(&specs).expect("slice specs");
    fs::write(specs.join("spec.md"), gap_delta()).expect("delta");
    fs::write(slice_dir.join("model.yaml"), gap_model()).expect("model");
    fs::write(
        slice_dir.join("metadata.yaml"),
        "target: mock\ntouched-specs:\n  - name: greeting\n    type: new\n",
    )
    .expect("metadata");
}

fn stage_wave_and_record(session: &Session, base: SnapshotId) {
    let layout = project::config::Layout::new(session.root());
    let wave = Wave::one_member(
        "demo",
        base.clone(),
        SliceName::from("greeting"),
        SnapshotId::from_digest(&"b".repeat(64)),
        vec![],
        EpochRef {
            writer: "local".into(),
            sequence: 0,
        },
    );
    let opened = wave.open(layout, ts()).expect("open wave");
    let record = project::build_record::BuildRecord::from_capture(
        project::snapshot::CodePatch {
            base: base.clone(),
            result: base,
            touched: vec![],
        },
        opened.digest,
        project::seam::wire::BuildReport {
            version: 1,
            slice: "greeting".into(),
            target: "mock@0.0.0".into(),
            status: project::seam::wire::BuildStatus::Success,
            findings: vec![],
            outputs: vec![],
            ui_surface: None,
            covered: vec![],
        },
        vec![],
    );
    record.write(&layout.slice_dir("greeting")).expect("write build record");
}

/// Acceptance 6 (fold half): unknown and conflict rows without
/// scenarios pass baseline coherence validation and land with status,
/// tags, both conflict arms, and final baseline ids.
#[tokio::test]
async fn gap_rows_fold_into_baseline_with_status_preserved() {
    let session = Session::scripted("mock", Vec::new());
    let snapshot = session.provider().freeze().await.expect("freeze");
    stage_wave_and_record(&session, snapshot);
    stage_built_gap_slice(&session);

    merge(&session, "greeting").await.expect("gap-status rows must not fail the scenario rule");

    let baseline = fs::read_to_string(session.root().join(".emery/specs/greeting/spec.md"))
        .expect("merged baseline");
    // Status preserved — the unknown stays [unknown]...
    assert!(baseline.contains("greeting error handling [unknown]"), "{baseline}");
    assert!(baseline.contains("Status: unknown"), "{baseline}");
    // ...and the conflict stays [conflict] with both arms' notes
    // intact (no downgrade to unknown).
    assert!(baseline.contains("session TTL [conflict]"), "{baseline}");
    assert!(baseline.contains("Status: conflict"), "{baseline}");
    assert!(baseline.contains("Note: docs says 30 minutes"), "{baseline}");
    assert!(baseline.contains("Note: code says 15 minutes"), "{baseline}");
    // Final baseline ids assigned like any other row.
    assert!(baseline.contains("ID: REQ-001"), "{baseline}");
    assert!(baseline.contains("ID: REQ-002"), "{baseline}");

    // No plan, no deferral facts: the wave fact snapshots no debt.
    let events = read_union(project::config::Layout::new(session.root())).expect("union");
    let deferred = events
        .iter()
        .find_map(|event| match &event.kind {
            EventKind::TargetMergeWaveCommitted { deferred, .. } => Some(deferred.clone()),
            _ => None,
        })
        .expect("target.merge.wave-committed");
    assert!(deferred.is_empty(), "standalone merge carries no deferred members: {deferred:?}");
}
