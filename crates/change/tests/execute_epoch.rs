//! RFC-86 S18 / RFC-91 D5: `plan.execute.started` at execute start
//! with exact per-leaf refinement digests on its typed `closed-plan`
//! coverage.

mod support;

use std::fs;

use change::Plan;
use change::plan::handlers::{Execute, ExecuteInput};
use mock::invoke::run;
use mock::session::Session;
use project::config::Layout;
use project::journal::{ClosedPlanCoverage, EventKind, read_union};
use support::plan_with_changes;

fn leaf(name: &str) -> change::Entry {
    support::change(name)
}

fn suite_answers() -> Vec<String> {
    vec![mock::answers::greeting_synthesis()]
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
    support::write_greeting_plan(session.root());
}

fn started_events(root: &std::path::Path) -> Vec<project::journal::Event> {
    read_union(Layout::new(root))
        .expect("union")
        .into_iter()
        .filter(|event| matches!(event.kind, EventKind::PlanExecuteStarted { .. }))
        .collect()
}

fn write_plan(root: &std::path::Path, plan: &Plan) {
    plan.save(&Layout::new(root).plan_path()).expect("save plan");
}

fn write_model(root: &std::path::Path, slice: &str, yaml: &str) {
    let dir = root.join(".emery/change/slices").join(slice);
    fs::create_dir_all(dir.join("specs")).expect("slice/specs");
    fs::write(dir.join("model.yaml"), yaml).expect("model.yaml");
    fs::write(
        dir.join("metadata.yaml"),
        "target: mock\ncreated-at: 2026-01-01T00:00:00Z\ndefined-at: 2026-01-01T00:00:01Z\n",
    )
    .expect("metadata");
}

#[tokio::test]
async fn appends_closed_epoch() {
    let session = Session::bare(suite_answers());
    let root = session.root().to_path_buf();
    scaffold_author(&session).await;
    support::refine_plan(&session).await;

    run::<Execute, _, _>(session.provider(), ExecuteInput::default())
        .await
        .expect("execute drains");

    let started = started_events(&root);
    assert_eq!(started.len(), 1, "exactly one plan.execute.started");
    let EventKind::PlanExecuteStarted {
        coverage:
            ClosedPlanCoverage::ClosedPlan {
                plan_digest,
                refinements,
            },
        discovery_digest,
    } = &started[0].kind
    else {
        panic!("expected PlanExecuteStarted");
    };
    assert!(plan_digest.starts_with("sha256:"), "{plan_digest}");
    assert!(discovery_digest.is_none());
    assert!(
        refinements.contains_key("greeting"),
        "covered leaf carries a refinement digest; got {refinements:?}"
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

/// Init the mock adapter.
async fn init_mock(session: &Session) {
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
}

/// The unknown-carrying single-slice fixture: refined `a` with a
/// staged refinement manifest, one open `[unknown]`, single-entry plan.
fn write_unknown_fixture(root: &std::path::Path) {
    write_model(
        root,
        "a",
        r"requirements:
  - id: REQ-003
    title: reset path not evidenced
    statement: ''
    status: unknown
    sources: [intent]
",
    );
    write_plan(root, &plan_with_changes(vec![leaf("a")]));
    // Execute requires a fresh manifest before it appends the epoch
    // (RFC-91 D5).
    support::stage_manifest(root, "a");
}

#[tokio::test]
async fn refined_leaf_covered() {
    let session = Session::bare(Vec::new());
    init_mock(&session).await;
    write_unknown_fixture(session.root());

    // Loop may stop after the epoch (open gap) — the fact must still
    // be recorded at start.
    drop(run::<Execute, _, _>(session.provider(), ExecuteInput::default()).await);

    let started = started_events(session.root());
    assert_eq!(started.len(), 1, "exactly one plan.execute.started");
    let EventKind::PlanExecuteStarted {
        coverage: ClosedPlanCoverage::ClosedPlan { refinements, .. },
        ..
    } = &started[0].kind
    else {
        panic!("expected PlanExecuteStarted");
    };
    assert!(
        refinements.contains_key("a"),
        "refined leaf → covered refinement digest; got {refinements:?}"
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

#[tokio::test]
async fn coverage_payload_hard_cut() {
    // Hard cut: the coverage wire shape carries `refinements`, never
    // the deleted `unknown-waivers` / `gap-policy` fields or the
    // deleted `refine-under-epoch` specs.
    let session = Session::bare(Vec::new());
    init_mock(&session).await;
    write_unknown_fixture(session.root());

    drop(run::<Execute, _, _>(session.provider(), ExecuteInput::default()).await);

    let started = started_events(session.root());
    let wire = serde_json::to_value(&started[0]).expect("serialize");
    let coverage = &wire["payload"]["coverage"];
    assert!(coverage.get("refinements").is_some(), "RFC-91: {coverage}");
    assert!(coverage.get("gap-policy").is_none(), "hard cut: {coverage}");
    assert!(coverage.get("unknown-waivers").is_none(), "hard cut: {coverage}");
    assert!(coverage.get("specs").is_none(), "hard cut: {coverage}");
}
