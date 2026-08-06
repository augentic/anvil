//! RFC-86 S19: execute gap gate before build — conflict / unknown /
//! waive / epoch-stale (`plan-gaps-unresolved`, `plan-epoch-stale`).

mod support;

use std::collections::BTreeMap;
use std::fs;

use change::orchestrate::enforce_before_build;
use change::plan::handlers::{Execute, ExecuteInput, WaiveSelector};
use diagnostics::digest::sha256_hex;
use jiff::Timestamp;
use mock::invoke::run;
use mock::session::Session;
use project::config::Layout;
use project::journal::{
    ClosedPlanCoverage, DEFAULT_ACTOR, Event, EventKind, LeafSpecCoverage, append_for, read_union,
};
use project::plan::{Plan, dir_cid};
use support::plan_with_changes;

/// Single-project plan entry so execute's workspace routing refusal
/// does not fire.
fn leaf(name: &str) -> change::Entry {
    let mut entry = support::change(name);
    entry.project = None;
    entry
}

fn err_code(err: &project::handler::Error) -> String {
    err.core().variant_str().into_owned()
}

fn write_plan(root: &std::path::Path, plan: &Plan) {
    plan.save(&Layout::new(root).plan_path()).expect("save plan");
}

fn write_refined(root: &std::path::Path, slice: &str, model: &str) {
    let dir = root.join(".emery/slices").join(slice);
    fs::create_dir_all(dir.join("specs")).expect("slice/specs");
    fs::write(dir.join("model.yaml"), model).expect("model.yaml");
    fs::write(
        dir.join("metadata.yaml"),
        "target: mock\ncreated-at: 2026-01-01T00:00:00Z\ndefined-at: 2026-01-01T00:00:01Z\n",
    )
    .expect("metadata");
}

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

fn stamp_epoch(
    root: &std::path::Path, plan_digest: &str, specs: BTreeMap<String, LeafSpecCoverage>,
    unknown_waivers: Vec<project::journal::UnknownWaiver>,
) {
    let ts = Timestamp::from_second(1_700_000_000).expect("timestamp");
    let event = Event::new(
        ts,
        EventKind::PlanExecuteStarted {
            coverage: ClosedPlanCoverage::ClosedPlan {
                plan_digest: plan_digest.into(),
                specs,
                unknown_waivers,
            },
            discovery_digest: None,
        },
    );
    append_for(Layout::new(root), DEFAULT_ACTOR, &[event]).expect("stamp epoch");
}

fn live_plan_digest(root: &std::path::Path) -> String {
    let bytes = fs::read(Layout::new(root).plan_path()).expect("plan.yaml");
    format!("sha256:{}", sha256_hex(&bytes))
}

#[tokio::test]
async fn conflict_blocks_build() {
    let session = Session::bare(Vec::new());
    init_mock(&session).await;
    write_refined(
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

    let err = run::<Execute, _, _>(session.provider(), ExecuteInput::default())
        .await
        .expect_err("conflict must refuse build");
    assert_eq!(err_code(&err), "plan-gaps-unresolved");
    let detail = err.core().to_string();
    assert!(detail.contains("conflict"), "{detail}");
    assert!(detail.contains("REQ-001"), "{detail}");
    assert!(detail.contains('a'), "inventory names the slice: {detail}");
}

#[tokio::test]
async fn unknown_blocks_without_waive() {
    let session = Session::bare(Vec::new());
    init_mock(&session).await;
    write_refined(
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

    let err = run::<Execute, _, _>(session.provider(), ExecuteInput::default())
        .await
        .expect_err("unknown must refuse build");
    assert_eq!(err_code(&err), "plan-gaps-unresolved");
    let detail = err.core().to_string();
    assert!(detail.contains("unknown"), "{detail}");
    assert!(detail.contains("--waive"), "{detail}");
}

#[tokio::test]
async fn waive_allows_gap_gate() {
    let session = Session::bare(Vec::new());
    init_mock(&session).await;
    write_refined(
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

    let err = run::<Execute, _, _>(
        session.provider(),
        ExecuteInput {
            waive: vec![WaiveSelector {
                slice: "a".into(),
                req: "REQ-003".into(),
            }],
            reason: Some("reset path deferred".into()),
        },
    )
    .await
    .expect_err("build may fail later without pins; gap gate must not");
    let code = err_code(&err);
    assert_ne!(code, "plan-gaps-unresolved", "waived unknown must pass gap gate: {err}");
    assert_ne!(code, "plan-epoch-stale", "fresh epoch must not be stale: {err}");

    let started = read_union(Layout::new(session.root()))
        .expect("union")
        .into_iter()
        .filter(|e| matches!(e.kind, EventKind::PlanExecuteStarted { .. }))
        .count();
    assert_eq!(started, 1, "epoch recorded before the post-gate failure");
}

#[tokio::test]
async fn divergence_alone_does_not_block() {
    let session = Session::bare(Vec::new());
    init_mock(&session).await;
    write_refined(
        session.root(),
        "a",
        r"requirements:
  - id: REQ-012
    title: retry budget: docs beat behaviour
    status: divergence
    sources: [intent]
",
    );
    write_plan(session.root(), &plan_with_changes(vec![leaf("a")]));

    let err = run::<Execute, _, _>(session.provider(), ExecuteInput::default())
        .await
        .expect_err("build may fail later without pins; divergence must not gate");
    let code = err_code(&err);
    assert_ne!(code, "plan-gaps-unresolved", "divergence is listed but allowed: {err}");
}

#[tokio::test]
async fn stale_existing_digest_refuses_build() {
    let session = Session::bare(Vec::new());
    init_mock(&session).await;
    write_refined(
        session.root(),
        "a",
        r"requirements:
  - id: REQ-001
    title: login works
    status: agreed
    sources: [intent]
",
    );
    let plan = plan_with_changes(vec![leaf("a")]);
    write_plan(session.root(), &plan);

    let layout = Layout::new(session.root());
    let plan_digest = live_plan_digest(session.root());
    let mut specs = BTreeMap::new();
    specs.insert(
        "a".into(),
        LeafSpecCoverage::Existing {
            digest: "sha256:deadbeef00000000000000000000000000000000000000000000000000000000"
                .into(),
        },
    );
    stamp_epoch(session.root(), &plan_digest, specs, Vec::new());

    // Live specs digest differs from the fabricated covering digest.
    let live = dir_cid(&layout.slice_dir("a").join("specs")).expect("live specs cid");
    assert!(
        live.to_string()
            != "sha256:deadbeef00000000000000000000000000000000000000000000000000000000"
    );

    let err = enforce_before_build(layout, &plan, "a").expect_err("stale epoch");
    assert_eq!(err.variant_str(), "plan-epoch-stale");
    let detail = err.to_string();
    assert!(detail.contains("drifted") || detail.contains("deadbeef"), "{detail}");
}

#[tokio::test]
async fn stale_plan_digest_refuses_build() {
    let session = Session::bare(Vec::new());
    init_mock(&session).await;
    write_refined(
        session.root(),
        "a",
        r"requirements:
  - id: REQ-001
    title: login works
    status: agreed
    sources: [intent]
",
    );
    let plan = plan_with_changes(vec![leaf("a")]);
    write_plan(session.root(), &plan);

    let layout = Layout::new(session.root());
    let live_specs = dir_cid(&layout.slice_dir("a").join("specs")).expect("specs cid").to_string();
    let mut specs = BTreeMap::new();
    specs.insert("a".into(), LeafSpecCoverage::Existing { digest: live_specs });
    stamp_epoch(
        session.root(),
        "sha256:0000000000000000000000000000000000000000000000000000000000000000",
        specs,
        Vec::new(),
    );

    let err = enforce_before_build(layout, &plan, "a").expect_err("stale plan digest");
    assert_eq!(err.variant_str(), "plan-epoch-stale");
    assert!(err.to_string().contains("plan.yaml"), "{}", err);
}
