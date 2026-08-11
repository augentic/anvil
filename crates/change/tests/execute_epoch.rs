//! RFC-86 S18 / RFC-86a D3 / RFC-91 D5: `plan.execute.started` at
//! execute start with exact per-leaf refinement digests and the
//! effective gap policy on its `closed-plan` coverage.

mod support;

use std::fs;

use change::Plan;
use change::plan::handlers::{Execute, ExecuteInput};
use mock::invoke::run;
use mock::session::Session;
use project::GapPolicy;
use project::config::Layout;
use project::journal::{ClosedPlanCoverage, EventKind, read_union};
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
                gap_policy,
            },
        discovery_digest,
    } = &started[0].kind
    else {
        panic!("expected PlanExecuteStarted");
    };
    assert!(plan_digest.starts_with("sha256:"), "{plan_digest}");
    assert!(discovery_digest.is_none());
    assert_eq!(*gap_policy, GapPolicy::Strict, "no flag, no declaration → strict");
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

/// Init the mock adapter with an optional `gap-policy` declaration.
async fn init_declared(session: &Session, gap_policy: Option<GapPolicy>) {
    run::<project::init::handlers::Init, _, _>(
        session.provider(),
        project::init::handlers::InitInput {
            adapter: Some("mock".to_string()),
            name: Some("demo".to_string()),
            gap_policy,
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

/// The effective policy on the sole `plan.execute.started` coverage.
fn started_policy(root: &std::path::Path) -> GapPolicy {
    let started = started_events(root);
    assert_eq!(started.len(), 1, "exactly one plan.execute.started");
    let EventKind::PlanExecuteStarted {
        coverage: ClosedPlanCoverage::ClosedPlan { gap_policy, .. },
        ..
    } = &started[0].kind
    else {
        panic!("expected PlanExecuteStarted");
    };
    *gap_policy
}

#[tokio::test]
async fn gap_policy_flag_rides_epoch_coverage() {
    let session = Session::bare(Vec::new());
    init_declared(&session, None).await;
    write_unknown_fixture(session.root());

    // Loop may stop after the epoch (open gap) — the fact must still
    // be recorded at start with the effective policy.
    drop(
        run::<Execute, _, _>(
            session.provider(),
            ExecuteInput {
                gap_policy: Some(GapPolicy::Defer),
            },
        )
        .await,
    );

    assert_eq!(started_policy(session.root()), GapPolicy::Defer, "flag rides the coverage");
    let started = started_events(session.root());
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
async fn project_declaration_resolves_without_flags() {
    let session = Session::bare(Vec::new());
    init_declared(&session, Some(GapPolicy::Defer)).await;
    write_unknown_fixture(session.root());

    drop(run::<Execute, _, _>(session.provider(), ExecuteInput::default()).await);

    assert_eq!(
        started_policy(session.root()),
        GapPolicy::Defer,
        "the project.yaml declaration is the effective policy when no flag is passed"
    );
}

#[tokio::test]
async fn flag_overrides_declaration_for_one_epoch() {
    let session = Session::bare(Vec::new());
    init_declared(&session, Some(GapPolicy::Defer)).await;
    write_unknown_fixture(session.root());

    drop(
        run::<Execute, _, _>(
            session.provider(),
            ExecuteInput {
                gap_policy: Some(GapPolicy::Strict),
            },
        )
        .await,
    );

    assert_eq!(
        started_policy(session.root()),
        GapPolicy::Strict,
        "the per-epoch flag overrides the declaration"
    );
}

#[tokio::test]
async fn no_coverage_payload_carries_unknown_waivers() {
    // Acceptance 9 (hard cut): the coverage wire shape carries
    // `gap-policy` + `refinements`, never the deleted `unknown-waivers`
    // field or `refine-under-epoch` specs.
    let session = Session::bare(Vec::new());
    init_declared(&session, None).await;
    write_unknown_fixture(session.root());

    drop(run::<Execute, _, _>(session.provider(), ExecuteInput::default()).await);

    let started = started_events(session.root());
    let wire = serde_json::to_value(&started[0]).expect("serialize");
    let coverage = &wire["payload"]["coverage"];
    assert_eq!(coverage["gap-policy"], "strict");
    assert!(coverage.get("refinements").is_some(), "RFC-91: {coverage}");
    assert!(coverage.get("unknown-waivers").is_none(), "hard cut: {coverage}");
    assert!(coverage.get("specs").is_none(), "hard cut: {coverage}");
}
