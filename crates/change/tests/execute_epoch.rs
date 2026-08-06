//! RFC-86 S18: `plan.execute.started` at execute start + `--waive`
//! validation (`plan-waiver-invalid`).

mod support;

use std::fs;

use change::Plan;
use change::plan::handlers::{Execute, ExecuteInput, WaiveSelector};
use mock::invoke::run;
use mock::session::Session;
use project::config::Layout;
use project::journal::{ClosedPlanCoverage, EventKind, LeafSpecCoverage, read_union};
use support::plan_with_changes;

/// Single-project plan entry (`project: None`) so execute's workspace
/// routing refusal does not fire.
fn leaf(name: &str) -> change::Entry {
    let mut entry = support::change(name);
    entry.project = None;
    entry
}

fn suite_answers() -> Vec<String> {
    vec![mock::answers::greeting_grouping(), mock::answers::greeting_synthesis()]
}

async fn scaffold_author(session: &Session) {
    run::<project::init::handlers::Init, _, _>(
        session.provider(),
        project::init::handlers::InitInput {
            adapter: Some("mock".to_string()),
            name: Some("demo".to_string()),
            ..Default::default()
        },
    )
    .await
    .expect("scaffold");
    run::<change::plan::handlers::Author, _, _>(
        session.provider(),
        change::plan::handlers::AuthorInput {
            name: "demo".to_string(),
            sources: support::greeting_binding(),
            intent: None,
            force: false,
        },
    )
    .await
    .expect("author");
}

fn started_events(root: &std::path::Path) -> Vec<project::journal::Event> {
    read_union(Layout::new(root))
        .expect("union")
        .into_iter()
        .filter(|event| matches!(event.kind, EventKind::PlanExecuteStarted { .. }))
        .collect()
}

fn err_code(err: &project::handler::Error) -> String {
    err.core().variant_str().into_owned()
}

fn write_plan(root: &std::path::Path, plan: &Plan) {
    plan.save(&Layout::new(root).plan_path()).expect("save plan");
}

fn write_model(root: &std::path::Path, slice: &str, yaml: &str) {
    let dir = root.join(".emery/slices").join(slice);
    fs::create_dir_all(dir.join("specs")).expect("slice/specs");
    fs::write(dir.join("model.yaml"), yaml).expect("model.yaml");
    fs::write(
        dir.join("metadata.yaml"),
        "target: mock\ncreated-at: 2026-01-01T00:00:00Z\ndefined-at: 2026-01-01T00:00:01Z\n",
    )
    .expect("metadata");
}

#[tokio::test]
async fn execute_appends_closed_plan_epoch() {
    let session = Session::bare(suite_answers());
    let root = session.root().to_path_buf();
    scaffold_author(&session).await;

    run::<Execute, _, _>(session.provider(), ExecuteInput::default())
        .await
        .expect("execute drains");

    let started = started_events(&root);
    assert_eq!(started.len(), 1, "exactly one plan.execute.started");
    let EventKind::PlanExecuteStarted {
        coverage:
            ClosedPlanCoverage::ClosedPlan {
                plan_digest,
                specs,
                unknown_waivers,
            },
        discovery_digest,
    } = &started[0].kind
    else {
        panic!("expected PlanExecuteStarted");
    };
    assert!(plan_digest.starts_with("sha256:"), "{plan_digest}");
    assert!(discovery_digest.is_none());
    assert!(unknown_waivers.is_empty());
    // At the moment the epoch is stamped, greeting has no specs yet
    // (refine runs later in the same execute), so coverage is
    // refine-under-epoch. Re-entry would see Existing after refine.
    assert_eq!(
        specs.get("greeting"),
        Some(&LeafSpecCoverage::RefineUnderEpoch),
        "unspec'd leaf at execute start → refine-under-epoch; got {specs:?}"
    );
    assert!(started[0].sequence >= 1);

    let status = run::<change::plan::handlers::Status, _, _>(
        session.provider(),
        change::plan::handlers::StatusInput {},
    )
    .await
    .expect("status");
    assert!(status.authorized, "epoch projects Authorized");
}

#[tokio::test]
async fn waive_without_reason_is_invalid() {
    let session = Session::bare(suite_answers());
    scaffold_author(&session).await;

    let err = run::<Execute, _, _>(
        session.provider(),
        ExecuteInput {
            waive: vec![WaiveSelector {
                slice: "greeting".into(),
                req: "REQ-001".into(),
            }],
            reason: None,
        },
    )
    .await
    .expect_err("missing --reason");
    assert_eq!(err_code(&err), "plan-waiver-invalid");
    assert!(started_events(session.root()).is_empty(), "no epoch on invalid waive");
}

#[tokio::test]
async fn reason_without_waive_is_invalid() {
    let session = Session::bare(suite_answers());
    scaffold_author(&session).await;

    let err = run::<Execute, _, _>(
        session.provider(),
        ExecuteInput {
            waive: Vec::new(),
            reason: Some("deferred".into()),
        },
    )
    .await
    .expect_err("orphan --reason");
    assert_eq!(err_code(&err), "plan-waiver-invalid");
}

#[tokio::test]
async fn waive_absent_gap_is_invalid() {
    let session = Session::bare(suite_answers());
    scaffold_author(&session).await;

    let err = run::<Execute, _, _>(
        session.provider(),
        ExecuteInput {
            waive: vec![WaiveSelector {
                slice: "greeting".into(),
                req: "REQ-999".into(),
            }],
            reason: Some("no such gap".into()),
        },
    )
    .await
    .expect_err("absent gap");
    assert_eq!(err_code(&err), "plan-waiver-invalid");
}

#[tokio::test]
async fn waive_conflict_is_invalid() {
    let session = Session::bare(Vec::new());
    run::<project::init::handlers::Init, _, _>(
        session.provider(),
        project::init::handlers::InitInput {
            adapter: Some("mock".to_string()),
            name: Some("demo".to_string()),
            ..Default::default()
        },
    )
    .await
    .expect("init");
    write_model(
        session.root(),
        "a",
        r"requirements:
  - id: REQ-001
    title: contradiction
    status: conflict
    sources: [intent]
",
    );
    write_plan(session.root(), &plan_with_changes(vec![leaf("a")]));

    let err = run::<Execute, _, _>(
        session.provider(),
        ExecuteInput {
            waive: vec![WaiveSelector {
                slice: "a".into(),
                req: "REQ-001".into(),
            }],
            reason: Some("cannot waive conflict".into()),
        },
    )
    .await
    .expect_err("conflict waive");
    assert_eq!(err_code(&err), "plan-waiver-invalid");
    let detail = err.core().to_string();
    assert!(detail.contains("conflict"), "detail should name conflict: {detail}");
    assert!(started_events(session.root()).is_empty());
}

#[tokio::test]
async fn valid_waive_nests_on_epoch_coverage() {
    let session = Session::bare(Vec::new());
    run::<project::init::handlers::Init, _, _>(
        session.provider(),
        project::init::handlers::InitInput {
            adapter: Some("mock".to_string()),
            name: Some("demo".to_string()),
            ..Default::default()
        },
    )
    .await
    .expect("init");
    write_model(
        session.root(),
        "a",
        r"requirements:
  - id: REQ-003
    title: reset path not evidenced
    status: unknown
    sources: [intent]
",
    );
    write_plan(session.root(), &plan_with_changes(vec![leaf("a")]));

    // Loop may stop after the epoch (build needs refine pins) — the
    // fact must still be recorded at start with nested waivers.
    drop(
        run::<Execute, _, _>(
            session.provider(),
            ExecuteInput {
                waive: vec![WaiveSelector {
                    slice: "a".into(),
                    req: "REQ-003".into(),
                }],
                reason: Some("reset path deferred".into()),
            },
        )
        .await,
    );

    let started = started_events(session.root());
    assert_eq!(started.len(), 1, "epoch recorded even when loop stops");
    let EventKind::PlanExecuteStarted {
        coverage:
            ClosedPlanCoverage::ClosedPlan {
                specs,
                unknown_waivers,
                ..
            },
        ..
    } = &started[0].kind
    else {
        panic!("expected PlanExecuteStarted");
    };
    assert_eq!(unknown_waivers.len(), 1);
    assert_eq!(unknown_waivers[0].slice, "a");
    assert_eq!(unknown_waivers[0].req, "REQ-003");
    assert_eq!(unknown_waivers[0].reason, "reset path deferred");
    assert!(
        matches!(specs.get("a"), Some(LeafSpecCoverage::Existing { .. })),
        "refined leaf → existing digest; got {specs:?}"
    );

    let status = run::<change::plan::handlers::Status, _, _>(
        session.provider(),
        change::plan::handlers::StatusInput {},
    )
    .await
    .expect("status");
    assert!(status.authorized);
    assert!(!status.ready, "open unknown keeps Ready false");
}
