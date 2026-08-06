//! Wave-commit identity maps + drifted-MODIFIED rejection (RFC-86 D5 / D9).

use std::fs;

use diagnostics::digest::sha256_hex;
use jiff::Timestamp;
use mock::invoke::run;
use mock::session::Session;
use project::journal::{EventKind, read_union};
use project::name::SliceName;
use project::seam::Workspaces as _;
use project::snapshot::SnapshotId;
use project::wave::{EpochRef, Wave};
use slice::BaselineIndex;
use slice::handlers::{MergeRun, MergeRunBody, MergeRunInput};

fn ts() -> Timestamp {
    Timestamp::from_second(1_700_000_000).expect("valid timestamp")
}

fn baseline_body() -> &'static str {
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

fn delta_with_local_ids() -> &'static str {
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
            actor: "local".into(),
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
        r#"version: 1
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
"#
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
async fn wave_commit_assigns_baseline_ids_and_records_maps() {
    let session = Session::scripted("mock", Vec::new());
    let snapshot = session.provider().freeze().await.expect("freeze");
    let wave_digest = stage_wave_and_record(&session, "login-flow", snapshot);
    stage_built_slice(&session, wave_digest.as_str());

    let body = run::<MergeRun, _, _>(
        session.provider(),
        MergeRunInput {
            name: "login-flow".into(),
            allow_composition_replace: false,
            preview: false,
            conflict_check: false,
        },
    )
    .await
    .expect("merge succeeds");
    let MergeRunBody::Merged(_) = body else {
        panic!("expected committed merge: {body:?}");
    };

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

#[tokio::test]
async fn drifted_modified_rejects_before_wave_committed() {
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

    let err = run::<MergeRun, _, _>(
        session.provider(),
        MergeRunInput {
            name: "login-flow".into(),
            allow_composition_replace: false,
            preview: false,
            conflict_check: false,
        },
    )
    .await
    .expect_err("drifted MODIFIED must fail");
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
async fn postflight_failure_keeps_wave_committed() {
    let session = Session::scripted("mock", Vec::new());
    let snapshot = session.provider().freeze().await.expect("freeze");
    let wave_digest = stage_wave_and_record(&session, "login-flow", snapshot);
    stage_built_slice(&session, wave_digest.as_str());
    fs::write(session.root().join(mock::behaviour::FAIL_MERGE_POSTFLIGHT_MARKER), "")
        .expect("postflight marker");

    let err = run::<MergeRun, _, _>(
        session.provider(),
        MergeRunInput {
            name: "login-flow".into(),
            allow_composition_replace: false,
            preview: false,
            conflict_check: false,
        },
    )
    .await
    .expect_err("postflight fails");
    assert!(err.to_string().contains("target-merge-postflight-failed"), "{err}");

    let layout = project::config::Layout::new(session.root());
    let events = read_union(layout).expect("union");
    assert!(
        events.iter().any(|e| matches!(e.kind, EventKind::TargetMergeWaveCommitted { .. })),
        "wave-committed before postflight"
    );
    assert!(
        events.iter().any(|e| matches!(e.kind, EventKind::TargetMergeWavePostflightFailed { .. })),
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
