//! Wave-commit identity maps + drifted-MODIFIED rejection (RFC-86 D5 / D9).

use std::fs;

use diagnostics::digest::sha256_hex;
use jiff::Timestamp;
use mock::session::Session;
use project::journal::{EventKind, read_union};
use project::name::SliceName;
use project::seam::Workspaces as _;
use project::snapshot::SnapshotId;
use project::wave::{EpochRef, Wave};
use slice::BaselineIndex;
use slice::orchestrate::MergeOutcome;

fn ts() -> Timestamp {
    Timestamp::from_second(1_700_000_000).expect("valid timestamp")
}

/// The merge phase over the session provider — what the execute loop
/// dispatches per built slice (standalone here: no `plan.yaml`, so the
/// claim gate self-skips).
async fn merge(session: &Session, slice: &str) -> Result<MergeOutcome, error::Error> {
    let layout = project::config::Layout::new(session.root());
    slice::orchestrate::merge(session.provider(), layout, ts(), slice, false).await
}

const fn baseline_body() -> &'static str {
    "### Requirement: User can log in\n\n\
     ID: REQ-007\n\n\
     Authentication via email and password only.\n\n\
     #### Scenario: Valid credentials\n\n\
     - GIVEN a registered user\n\
     - WHEN they submit correct credentials\n\
     - THEN they receive a session token\n\n\
     ### Requirement: User can log out\n\n\
     ID: REQ-008\n\n\
     Session invalidation on logout.\n\n\
     #### Scenario: Active session\n\n\
     - GIVEN an authenticated user\n\
     - WHEN they log out\n\
     - THEN the session is invalidated\n"
}

const fn delta_with_local_ids() -> &'static str {
    "# Auth slice\n\n\
     ## MODIFIED Requirements\n\n\
     ### Requirement: User can log in\n\n\
     ID: REQ-001\n\n\
     Authentication via email/password *or* passkey.\n\n\
     #### Scenario: Valid credentials\n\n\
     - GIVEN a registered user\n\
     - WHEN they submit correct credentials\n\
     - THEN they receive a session token\n\n\
     #### Scenario: Passkey login\n\n\
     - GIVEN a registered user with a passkey\n\
     - WHEN they authenticate via passkey\n\
     - THEN they receive a session token\n\n\
     ## ADDED Requirements\n\n\
     ### Requirement: Password reset entry\n\n\
     ID: REQ-002\n\n\
     A password reset entry exists on the login screen.\n\n\
     #### Scenario: Open reset\n\n\
     - GIVEN the login screen\n\
     - WHEN the user opens reset\n\
     - THEN the reset flow starts\n"
}

fn stage_wave_and_record(session: &Session, slice: &str, base: SnapshotId) -> SnapshotId {
    let layout = project::config::Layout::new(session.root());
    let wave = Wave::one_member(
        "demo",
        base.clone(),
        SliceName::from(slice),
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
        opened.digest.clone(),
        project::seam::wire::BuildReport {
            version: 1,
            slice: slice.into(),
            target: "mock@0.0.0".into(),
            status: project::seam::wire::BuildStatus::Success,
            findings: vec![],
            outputs: vec![],
            ui_surface: None,
        },
    );
    record.write(&layout.slice_dir(slice)).expect("write build record");
    opened.digest
}

fn stage_built_slice(session: &Session, digest: &str) {
    let root = session.root();
    let slice_dir = root.join(".emery/slices/login-flow");
    let specs = slice_dir.join("specs/auth");
    fs::create_dir_all(&specs).expect("slice specs");
    fs::create_dir_all(root.join(".emery/specs/auth")).expect("baseline specs");
    fs::write(root.join(".emery/specs/auth/spec.md"), baseline_body()).expect("baseline");
    fs::write(specs.join("spec.md"), delta_with_local_ids()).expect("delta");
    fs::write(
        slice_dir.join("metadata.yaml"),
        "target: mock\ntouched-specs:\n  - name: auth\n    type: modified\n",
    )
    .expect("metadata");

    let index = BaselineIndex::build(&root.join(".emery/specs")).expect("baseline index");
    let body = index.body("auth", "REQ-007").expect("baseline body");
    let baseline_digest = format!("sha256:{}", sha256_hex(body.as_bytes()));

    let model = format!(
        r"version: 1
slice: login-flow
requirements:
  - id: REQ-001
    title: User can log in
    status: agreed
    domain: auth
    baseline-id: REQ-007
    baseline-digest: {baseline_digest}
    sources: [docs]
    claims:
      - source: docs
        id: login.flow
        kind: requirement
    statement: Authentication via email/password or passkey.
    scenarios:
      - Passkey login
  - id: REQ-002
    title: Password reset entry
    status: agreed
    domain: auth
    sources: [docs]
    claims:
      - source: docs
        id: reset.entry
        kind: requirement
    statement: A password reset entry exists on the login screen.
    scenarios:
      - Open reset
tasks:
  - id: TASK-001
    text: Wire passkey login.
    satisfies: [REQ-001]
  - id: TASK-002
    text: Wire reset entry.
    satisfies: [REQ-002]
"
    );
    fs::write(slice_dir.join("model.yaml"), model).expect("model");
    fs::write(
        slice_dir.join("tasks.md"),
        "# Tasks\n\n- TASK-001 satisfies REQ-001\n- TASK-002 satisfies REQ-002\n",
    )
    .expect("tasks");
    let _ = digest;
}

#[tokio::test]
async fn wave_commit_assigns() {
    let session = Session::scripted("mock", Vec::new());
    let snapshot = session.provider().freeze().await.expect("freeze");
    let wave_digest = stage_wave_and_record(&session, "login-flow", snapshot);
    stage_built_slice(&session, wave_digest.as_str());

    merge(&session, "login-flow").await.expect("merge succeeds");

    let merged = fs::read_to_string(session.root().join(".emery/specs/auth/spec.md"))
        .expect("merged baseline");
    // MODIFIED keeps baseline REQ-007; ADDED takes the next free number (REQ-009).
    assert!(merged.contains("ID: REQ-007"), "{merged}");
    assert!(merged.contains("ID: REQ-008"), "{merged}");
    assert!(merged.contains("ID: REQ-009"), "{merged}");
    assert!(!merged.contains("ID: REQ-001\n"), "{merged}");
    assert!(merged.contains("passkey"), "{merged}");
    assert!(merged.contains("Password reset entry"), "{merged}");

    let layout = project::config::Layout::new(session.root());
    let events = read_union(layout).expect("union");
    let committed = events.iter().find_map(|event| match &event.kind {
        EventKind::TargetMergeWaveCommitted {
            digest,
            slice_name,
            identity_maps,
            ..
        } => {
            assert_eq!(digest, wave_digest.as_str());
            assert_eq!(slice_name.as_str(), "login-flow");
            Some(identity_maps.clone())
        }
        _ => None,
    });
    let maps = committed.expect("target.merge.wave-committed");
    assert_eq!(maps.len(), 2, "{maps:?}");
    assert_eq!(maps[0].local, "REQ-001");
    assert_eq!(maps[0].baseline, "REQ-007");
    assert_eq!(maps[1].local, "REQ-002");
    assert_eq!(maps[1].baseline, "REQ-009");

    assert!(
        events.iter().any(|e| matches!(e.kind, EventKind::TargetMergeWaveSucceeded { .. })),
        "wave-succeeded after postflight"
    );
}

/// Acceptance #3 — two slices refined against the same baseline merge
/// without requirement-id collision (each keeps slice-local `REQ-001`
/// until wave commit assigns distinct baseline numbers).
#[tokio::test]
async fn two_slices_merge_without() {
    let session = Session::scripted("mock", Vec::new());
    let root = session.root();
    fs::create_dir_all(root.join(".emery/specs/auth")).expect("baseline");
    fs::write(root.join(".emery/specs/auth/spec.md"), baseline_body()).expect("baseline body");

    let added_a = "# Slice A\n\n## ADDED Requirements\n\n\
         ### Requirement: Passkey login\n\n\
         ID: REQ-001\n\n\
         Passkey authentication.\n\n\
         #### Scenario: Passkey\n\n\
         - GIVEN a passkey\n\
         - WHEN the user authenticates\n\
         - THEN a session starts\n";
    let added_b = "# Slice B\n\n## ADDED Requirements\n\n\
         ### Requirement: Reset entry\n\n\
         ID: REQ-001\n\n\
         Password reset entry on login.\n\n\
         #### Scenario: Open reset\n\n\
         - GIVEN the login screen\n\
         - WHEN the user opens reset\n\
         - THEN the reset flow starts\n";

    for (slice, delta, title) in
        [("slice-a", added_a, "Passkey login"), ("slice-b", added_b, "Reset entry")]
    {
        let snapshot = session.provider().freeze().await.expect("freeze");
        stage_wave_and_record(&session, slice, snapshot);
        let slice_dir = root.join(".emery/slices").join(slice);
        let specs = slice_dir.join("specs/auth");
        fs::create_dir_all(&specs).expect("specs");
        fs::write(specs.join("spec.md"), delta).expect("delta");
        fs::write(
            slice_dir.join("metadata.yaml"),
            "target: mock\ntouched-specs:\n  - name: auth\n    type: new\n",
        )
        .expect("metadata");
        fs::write(
            slice_dir.join("model.yaml"),
            format!(
                r"version: 1
slice: {slice}
requirements:
  - id: REQ-001
    title: {title}
    status: agreed
    domain: auth
    sources: [docs]
    claims:
      - source: docs
        id: {slice}.claim
        kind: requirement
    statement: {title}.
    scenarios:
      - scenario
tasks:
  - id: TASK-001
    text: Implement {title}.
    satisfies: [REQ-001]
"
            ),
        )
        .expect("model");
        fs::write(slice_dir.join("tasks.md"), "# Tasks\n\n- TASK-001 satisfies REQ-001\n")
            .expect("tasks");

        merge(&session, slice).await.unwrap_or_else(|err| panic!("merge {slice}: {err}"));
    }

    let merged = fs::read_to_string(root.join(".emery/specs/auth/spec.md")).expect("merged");
    assert!(merged.contains("ID: REQ-007"), "{merged}");
    assert!(merged.contains("ID: REQ-008"), "{merged}");
    // Both ADDED rows take distinct next-free baseline ids.
    assert!(merged.contains("ID: REQ-009"), "{merged}");
    assert!(merged.contains("ID: REQ-010"), "{merged}");
    assert!(merged.contains("Passkey"), "{merged}");
    assert!(merged.contains("Reset entry") || merged.contains("reset"), "{merged}");

    let layout = project::config::Layout::new(root);
    let events = read_union(layout).expect("union");
    let maps: Vec<_> = events
        .iter()
        .filter_map(|event| match &event.kind {
            EventKind::TargetMergeWaveCommitted { identity_maps, .. } => Some(identity_maps),
            _ => None,
        })
        .collect();
    assert_eq!(maps.len(), 2, "one wave-committed per slice: {maps:?}");
    let baselines: Vec<&str> =
        maps.iter().flat_map(|m| m.iter().map(|row| row.baseline.as_str())).collect();
    assert!(baselines.contains(&"REQ-009"), "{baselines:?}");
    assert!(baselines.contains(&"REQ-010"), "{baselines:?}");
    assert_eq!(baselines.len(), 2, "no shared baseline id: {baselines:?}");
}

#[tokio::test]
async fn drifted_modified_rejects() {
    let session = Session::scripted("mock", Vec::new());
    let snapshot = session.provider().freeze().await.expect("freeze");
    stage_wave_and_record(&session, "login-flow", snapshot);
    stage_built_slice(&session, "");

    // Drift the live baseline body after refine recorded its digest.
    fs::write(
        session.root().join(".emery/specs/auth/spec.md"),
        "### Requirement: User can log in\n\n\
         ID: REQ-007\n\n\
         Authentication via email and password — DRIFTED.\n\n\
         #### Scenario: Valid credentials\n\n\
         - GIVEN a registered user\n\
         - WHEN they submit correct credentials\n\
         - THEN they receive a session token\n\n\
         ### Requirement: User can log out\n\n\
         ID: REQ-008\n\n\
         Session invalidation on logout.\n\n\
         #### Scenario: Active session\n\n\
         - GIVEN an authenticated user\n\
         - WHEN they log out\n\
         - THEN the session is invalidated\n",
    )
    .expect("drift baseline");

    let err = merge(&session, "login-flow").await.expect_err("drifted MODIFIED must fail");
    let text = err.to_string();
    assert!(text.contains("merge-base-drifted"), "{text}");

    // Failures before wave-committed leave no merged projection / no baseline fold.
    let layout = project::config::Layout::new(session.root());
    let events = read_union(layout).expect("union");
    assert!(
        !events.iter().any(|e| matches!(e.kind, EventKind::TargetMergeWaveCommitted { .. })),
        "no wave-committed on pre-commit failure"
    );
    assert!(
        session.root().join(".emery/slices/login-flow").exists(),
        "slice still present (not archived)"
    );
    let baseline = fs::read_to_string(session.root().join(".emery/specs/auth/spec.md"))
        .expect("baseline still drifted original");
    assert!(baseline.contains("DRIFTED"), "{baseline}");
    assert!(!baseline.contains("passkey"), "{baseline}");
}

#[tokio::test]
async fn postflight_failure_keeps() {
    let session = Session::scripted("mock", Vec::new());
    let snapshot = session.provider().freeze().await.expect("freeze");
    let wave_digest = stage_wave_and_record(&session, "login-flow", snapshot);
    stage_built_slice(&session, wave_digest.as_str());
    fs::write(session.root().join(mock::behaviour::FAIL_MERGE), "").expect("postflight marker");

    let err = merge(&session, "login-flow").await.expect_err("postflight fails");
    assert!(err.to_string().contains("target-merge-postflight-failed"), "{err}");

    let layout = project::config::Layout::new(session.root());
    let events = read_union(layout).expect("union");
    assert!(
        events.iter().any(|e| matches!(e.kind, EventKind::TargetMergeWaveCommitted { .. })),
        "wave-committed before postflight"
    );
    assert!(
        events.iter().any(|e| matches!(e.kind, EventKind::MergeWavePostflightFailed { .. })),
        "wave-postflight-failed"
    );
    assert!(
        !events.iter().any(|e| matches!(e.kind, EventKind::TargetMergeWaveSucceeded { .. })),
        "no wave-succeeded"
    );
    // Non-rollback: baseline fold stands with finalized ids.
    let merged = fs::read_to_string(session.root().join(".emery/specs/auth/spec.md"))
        .expect("merged baseline");
    assert!(merged.contains("ID: REQ-009"), "{merged}");
    assert!(!session.root().join(".emery/slices/login-flow").exists());
}
