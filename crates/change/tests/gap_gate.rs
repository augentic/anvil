//! RFC-86 S19 / RFC-86a: execute gap gate before build — open
//! conflict / unknown rows are dispositioned at the gate itself
//! (policy deferrals, unconditional), durable deferrals cover across
//! epochs, epoch staleness (`plan-epoch-stale`).

mod support;

use std::collections::BTreeMap;
use std::fs;

use change::orchestrate::enforce_before_build;
use change::plan::handlers::{Execute, ExecuteInput, Gaps, GapsInput};
use diagnostics::digest::sha256_hex;
use jiff::Timestamp;
use mock::invoke::run;
use mock::session::Session;
use project::config::Layout;
use project::journal::{
    ClosedPlanCoverage, DEFAULT_WRITER, Event, EventKind, LeafSpecCoverage, append_for, read_union,
};
use project::plan::{Disposition, Plan, dir_cid};
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

/// `(slice, req, reason)` of every gate-minted deferral fact in the
/// journal union, told apart from pre-seeded facts by the gate's
/// synthesized epoch reason.
fn gate_minted_facts(root: &std::path::Path) -> Vec<(String, String, String)> {
    read_union(Layout::new(root))
        .expect("union")
        .into_iter()
        .filter_map(|event| match event.kind {
            EventKind::GapDeferred {
                slice, req, reason, ..
            } if reason.starts_with("deferred at the build gate under epoch ") => {
                Some((slice.as_str().to_string(), req, reason))
            }
            _ => None,
        })
        .collect()
}

/// Cover one `<slice>/<req>` with a pre-existing durable deferral
/// fact — standing in for an earlier gate-time mint. Its reason stays
/// distinguishable from the gate's synthesized epoch reason.
async fn defer(session: &Session, slice: &str, req: &str, reason: &str) {
    let gaps = run::<Gaps, _, _>(session.provider(), GapsInput {}).await.expect("gaps");
    let row = gaps
        .rows
        .iter()
        .find(|row| row.slice == slice && row.req == req)
        .unwrap_or_else(|| panic!("gap row `{slice}/{req}` in the live inventory"));
    let digest = row.requirement_digest.clone().expect("digest-bearing row");
    let event = Event::new(
        Timestamp::now(),
        EventKind::GapDeferred {
            slice: slice.into(),
            req: req.into(),
            requirement_digest: digest,
            reason: reason.into(),
        },
    );
    append_for(Layout::new(session.root()), DEFAULT_WRITER, &[event]).expect("append deferral");
}

fn stamp_epoch(
    root: &std::path::Path, plan_digest: &str, specs: BTreeMap<String, LeafSpecCoverage>,
) {
    let ts = Timestamp::from_second(1_700_000_000).expect("timestamp");
    let event = Event::new(
        ts,
        EventKind::PlanExecuteStarted {
            coverage: ClosedPlanCoverage::ClosedPlan {
                plan_digest: plan_digest.into(),
                specs,
            },
            discovery_digest: None,
        },
    );
    append_for(Layout::new(root), DEFAULT_WRITER, &[event]).expect("stamp epoch");
}

fn live_plan_digest(root: &std::path::Path) -> String {
    let bytes = fs::read(Layout::new(root).plan_path()).expect("plan.yaml");
    format!("sha256:{}", sha256_hex(&bytes))
}

#[tokio::test]
async fn open_unknown_deferred_at_gate() {
    // The gate dispositions open rows itself — one `gap.deferred`
    // fact with the synthesized epoch reason — and build proceeds
    // (the post-gate failure here is the missing base pins, a stop).
    let session = Session::bare(Vec::new());
    init_mock(&session).await;
    write_refined(
        session.root(),
        "a",
        r"requirements:
  - id: REQ-003
    title: reset path not evidenced
    statement: ''
    status: unknown
    sources: [intent]
",
    );
    write_plan(session.root(), &plan_with_changes(vec![leaf("a")]));

    let err = run::<Execute, _, _>(session.provider(), ExecuteInput::default())
        .await
        .expect_err("build fails later without pins; the gap gate must not");
    assert_eq!(err_code(&err), "plan-execute-stopped", "open unknown never gates: {err}");

    let facts = gate_minted_facts(session.root());
    assert_eq!(facts.len(), 1, "one policy fact per open row: {facts:?}");
    let (slice, req, reason) = &facts[0];
    assert_eq!(slice, "a");
    assert_eq!(req, "REQ-003");
    assert!(
        reason.starts_with("deferred at the build gate under epoch "),
        "synthesized policy reason: {reason}"
    );

    // The minted disposition is visible in the projected inventory.
    let gaps = run::<Gaps, _, _>(session.provider(), GapsInput {}).await.expect("gaps");
    assert_eq!(gaps.rows.len(), 1);
    assert_eq!(gaps.rows[0].disposition, Some(Disposition::Deferred), "{:?}", gaps.rows);
}

#[tokio::test]
async fn open_conflict_deferred_at_gate() {
    // D6: conflicts defer under the same gate-time minting —
    // build-over stays forbidden, exclusion proceeds.
    let session = Session::bare(Vec::new());
    init_mock(&session).await;
    write_refined(
        session.root(),
        "a",
        r"requirements:
  - id: REQ-001
    title: contradiction
    statement: ''
    status: conflict
    sources: [intent]
",
    );
    write_plan(session.root(), &plan_with_changes(vec![leaf("a")]));

    let err = run::<Execute, _, _>(session.provider(), ExecuteInput::default())
        .await
        .expect_err("build fails later without pins; the gap gate must not");
    assert_eq!(err_code(&err), "plan-execute-stopped", "open conflict never gates: {err}");

    let facts = gate_minted_facts(session.root());
    assert_eq!(facts.len(), 1, "one policy fact for the conflict: {facts:?}");
    assert_eq!(facts[0].1, "REQ-001");

    let gaps = run::<Gaps, _, _>(session.provider(), GapsInput {}).await.expect("gaps");
    assert_eq!(gaps.rows[0].disposition, Some(Disposition::Deferred), "{:?}", gaps.rows);
}

#[tokio::test]
async fn digest_less_open_row_refused_at_gate() {
    // A legacy `spec.md`-fallback inventory (refined slice, model
    // without requirements) carries no requirement digests — no fact
    // can take its open rows out of build scope, so the gate refuses
    // instead of building over the gap silently.
    let session = Session::bare(Vec::new());
    init_mock(&session).await;
    write_refined(session.root(), "a", "requirements: []\n");
    let specs = session.root().join(".emery/slices/a/specs/auth");
    fs::create_dir_all(&specs).expect("specs dir");
    fs::write(
        specs.join("spec.md"),
        "### Requirement: reset path not evidenced [unknown]\n\
         ID: REQ-001\n\
         Sources: []\n\
         Status: unknown\n",
    )
    .expect("spec.md");
    write_plan(session.root(), &plan_with_changes(vec![leaf("a")]));

    let err = run::<Execute, _, _>(session.provider(), ExecuteInput::default())
        .await
        .expect_err("a digest-less open row must refuse at the gate");
    assert_eq!(err_code(&err), "plan-gap-digest-missing", "{err}");
    let detail = err.core().to_string();
    assert!(detail.contains("a/REQ-001"), "the detail names the row: {detail}");
    assert!(
        gate_minted_facts(session.root()).is_empty(),
        "the refusal mints nothing — no fact without a match key"
    );
}

#[tokio::test]
async fn deferred_unknown_passes_gap_gate() {
    let session = Session::bare(Vec::new());
    init_mock(&session).await;
    write_refined(
        session.root(),
        "a",
        r"requirements:
  - id: REQ-003
    title: reset path not evidenced
    statement: ''
    status: unknown
    sources: [intent]
",
    );
    write_plan(session.root(), &plan_with_changes(vec![leaf("a")]));
    defer(&session, "a", "REQ-003", "reset path deferred").await;

    let err = run::<Execute, _, _>(session.provider(), ExecuteInput::default())
        .await
        .expect_err("build may fail later without pins; gap gate must not");
    assert_eq!(err_code(&err), "plan-execute-stopped", "fresh epoch must not be stale: {err}");
    assert!(
        gate_minted_facts(session.root()).is_empty(),
        "the pre-existing fact covers — the gate mints nothing"
    );

    let started = read_union(Layout::new(session.root()))
        .expect("union")
        .into_iter()
        .filter(|e| matches!(e.kind, EventKind::PlanExecuteStarted { .. }))
        .count();
    assert_eq!(started, 1, "epoch recorded before the post-gate failure");
}

#[tokio::test]
async fn deferred_conflict_passes_gap_gate() {
    // D6: `[conflict]` defers under the same exclusion semantics —
    // build-over stays forbidden, exclusion proceeds.
    let session = Session::bare(Vec::new());
    init_mock(&session).await;
    write_refined(
        session.root(),
        "a",
        r"requirements:
  - id: REQ-001
    title: contradiction
    statement: ''
    status: conflict
    sources: [intent]
",
    );
    write_plan(session.root(), &plan_with_changes(vec![leaf("a")]));
    defer(&session, "a", "REQ-001", "tie deferred to next change").await;

    let err = run::<Execute, _, _>(session.provider(), ExecuteInput::default())
        .await
        .expect_err("build may fail later without pins; gap gate must not");
    assert_eq!(err_code(&err), "plan-execute-stopped", "{err}");
    assert!(
        gate_minted_facts(session.root()).is_empty(),
        "the pre-existing fact covers — the gate mints nothing"
    );
}

#[tokio::test]
async fn deferral_covers_fresh_epoch_without_resupply() {
    // Acceptance 2 / 9: the disposition is durable — a resume opens a
    // fresh epoch with no flags and the covering fact still holds; no
    // path demands re-supplying a decision already on the log.
    let session = Session::bare(Vec::new());
    init_mock(&session).await;
    write_refined(
        session.root(),
        "a",
        r"requirements:
  - id: REQ-003
    title: reset path not evidenced
    statement: ''
    status: unknown
    sources: [intent]
",
    );
    write_plan(session.root(), &plan_with_changes(vec![leaf("a")]));
    defer(&session, "a", "REQ-003", "reset path deferred").await;

    // First run fails later (no base pins) — not the behavior under test.
    drop(run::<Execute, _, _>(session.provider(), ExecuteInput::default()).await);

    // Resume with no flags opens a fresh epoch: the fact still covers.
    let err = run::<Execute, _, _>(session.provider(), ExecuteInput::default())
        .await
        .expect_err("build may fail later without pins; gap gate must not");
    assert_eq!(err_code(&err), "plan-execute-stopped", "{err}");
    assert!(
        gate_minted_facts(session.root()).is_empty(),
        "the durable deferral covers both epochs — no gate-time mint"
    );

    let started = read_union(Layout::new(session.root()))
        .expect("union")
        .into_iter()
        .filter(|e| matches!(e.kind, EventKind::PlanExecuteStarted { .. }))
        .count();
    assert_eq!(started, 2, "each non-drained execute opens its own epoch");
}

#[tokio::test]
async fn digest_lapsed_deferral_reminted_at_gate() {
    // Acceptance 2: a re-refine that changes the requirement body
    // lapses the deferral — the new row is open, and the gate
    // dispositions it afresh under the new digest.
    let session = Session::bare(Vec::new());
    init_mock(&session).await;
    write_refined(
        session.root(),
        "a",
        r"requirements:
  - id: REQ-003
    title: reset path not evidenced
    statement: ''
    status: unknown
    sources: [intent]
",
    );
    write_plan(session.root(), &plan_with_changes(vec![leaf("a")]));
    defer(&session, "a", "REQ-003", "reset path deferred").await;

    // The requirement body changes (new evidence reshaped the gap):
    // the recorded digest no longer matches — the deferral lapses.
    write_refined(
        session.root(),
        "a",
        r"requirements:
  - id: REQ-003
    title: reset path partially evidenced
    statement: ''
    status: unknown
    sources: [intent]
",
    );

    let err = run::<Execute, _, _>(session.provider(), ExecuteInput::default())
        .await
        .expect_err("build fails later without pins; the gap gate must not");
    assert_eq!(err_code(&err), "plan-execute-stopped", "{err}");

    let facts = gate_minted_facts(session.root());
    assert_eq!(facts.len(), 1, "the lapsed row is re-minted at the gate: {facts:?}");
    assert!(
        facts[0].2.starts_with("deferred at the build gate under epoch "),
        "gate-time fact, not the pre-seeded one: {}",
        facts[0].2
    );
    let gaps = run::<Gaps, _, _>(session.provider(), GapsInput {}).await.expect("gaps");
    assert_eq!(gaps.rows[0].disposition, Some(Disposition::Deferred), "{:?}", gaps.rows);
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
    title: 'retry budget: docs beat behaviour'
    statement: ''
    status: divergence
    sources: [intent]
",
    );
    write_plan(session.root(), &plan_with_changes(vec![leaf("a")]));

    let err = run::<Execute, _, _>(session.provider(), ExecuteInput::default())
        .await
        .expect_err("build may fail later without pins; divergence must not gate");
    assert_eq!(err_code(&err), "plan-execute-stopped", "{err}");
    assert!(
        gate_minted_facts(session.root()).is_empty(),
        "divergence takes no disposition — nothing to mint"
    );
}

#[tokio::test]
async fn concurrent_stale_epoch_refuses_build_via_execute() {
    let session = Session::bare(Vec::new());
    init_mock(&session).await;
    write_refined(
        session.root(),
        "a",
        r"requirements:
  - id: REQ-001
    title: login works
    statement: ''
    status: agreed
    sources: [intent]
",
    );
    write_plan(session.root(), &plan_with_changes(vec![leaf("a")]));

    // A concurrent writer stamps an epoch *after* this run opens its
    // own (simulated with a later timestamp): the newest epoch governs
    // the gate, and its coverage no longer matches the live plan, so
    // the loop's build phase must refuse — through the full `Execute`
    // boundary, not the kernel.
    let peer_epoch = Event::new(
        Timestamp::from_second(4_102_444_800).expect("timestamp"),
        EventKind::PlanExecuteStarted {
            coverage: ClosedPlanCoverage::ClosedPlan {
                plan_digest:
                    "sha256:0000000000000000000000000000000000000000000000000000000000000000".into(),
                specs: BTreeMap::new(),
            },
            discovery_digest: None,
        },
    );
    append_for(Layout::new(session.root()), "peer", &[peer_epoch]).expect("peer epoch");

    let err = run::<Execute, _, _>(session.provider(), ExecuteInput::default())
        .await
        .expect_err("stale covering epoch must refuse build");
    assert_eq!(err_code(&err), "plan-epoch-stale");
    assert!(err.core().to_string().contains("plan.yaml"), "{err}");
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
    statement: ''
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
    stamp_epoch(session.root(), &plan_digest, specs);

    // Live specs digest differs from the fabricated covering digest.
    let live = dir_cid(&layout.slice_dir("a").join("specs")).expect("live specs cid");
    assert!(
        live.to_string()
            != "sha256:deadbeef00000000000000000000000000000000000000000000000000000000"
    );

    let err = enforce_before_build(
        layout,
        &plan,
        "a",
        Timestamp::from_second(1_700_000_100).expect("timestamp"),
    )
    .expect_err("stale epoch");
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
    statement: ''
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
    );

    let err = enforce_before_build(
        layout,
        &plan,
        "a",
        Timestamp::from_second(1_700_000_100).expect("timestamp"),
    )
    .expect_err("stale plan digest");
    assert_eq!(err.variant_str(), "plan-epoch-stale");
    assert!(err.to_string().contains("plan.yaml"), "{}", err);
}
