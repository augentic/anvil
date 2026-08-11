//! RFC-86 Phase C remaining acceptance fixtures (S21).
//!
//! Covers Acceptance #8–15 leftovers called out in the appendix test
//! list that earlier Phase C sessions did not land as dedicated
//! end-to-end fixtures:
//!
//! - in-scope drop (D24 / #13)
//! - coverage wire shape + stale epoch (#8)
//! - Ready skipped when deferring; clear → Ready (D22 / #12)
//! - post-author resume naming (D26 / #14)
//! - one-member wave opened before build under execute (D9 / #15)

mod support;

use std::collections::BTreeMap;
use std::fs;

use change::orchestrate::enforce_before_build;
use change::plan::handlers::{
    Author, AuthorInput, Defer, DeferInput, DeferSelector, Execute, ExecuteInput, Gaps, GapsInput,
    Status, StatusInput,
};
use change::{LoopStep, Plan};
use diagnostics::digest::sha256_hex;
use jiff::Timestamp;
use mock::behaviour;
use mock::invoke::run;
use mock::session::Session;
use project::GapPolicy;
use project::config::Layout;
use project::journal::{
    ClosedPlanCoverage, DEFAULT_WRITER, Event, EventKind, LeafSpecCoverage, append_for, read_union,
};
use project::plan::dir_cid;
use support::plan_with_changes;

fn leaf(name: &str) -> change::Entry {
    let mut entry = support::change(name);
    entry.project = None;
    entry
}

fn write_plan(root: &std::path::Path, plan: &Plan) {
    plan.save(&Layout::new(root).plan_path()).expect("save plan");
}

fn write_refined(root: &std::path::Path, slice: &str, model: &str) {
    write_refined_meta(root, slice, model, false);
}

fn write_refined_meta(root: &std::path::Path, slice: &str, model: &str, dropped: bool) {
    let dir = root.join(".emery/slices").join(slice);
    fs::create_dir_all(dir.join("specs")).expect("slice/specs");
    fs::write(dir.join("model.yaml"), model).expect("model.yaml");
    let mut meta = String::from(
        "target: mock\ncreated-at: 2026-01-01T00:00:00Z\ndefined-at: 2026-01-01T00:00:01Z\n",
    );
    if dropped {
        // Membership exclusion is `dropped_at` on the live tree (D24).
        // Prior fixtures keep the dir so `in_scope` can read the stamp;
        // `slice drop`'s archive move is the abandon path, not the
        // Ready/gaps membership signal under test here.
        meta.push_str("dropped-at: \"2026-01-02T00:00:00Z\"\n");
    }
    fs::write(dir.join("metadata.yaml"), meta).expect("metadata");
    // Specs tree so `dir_cid` is stable for Existing coverage stamps.
    fs::write(dir.join("specs").join(".keep"), "").expect("specs keep");
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

fn suite_answers() -> Vec<String> {
    vec![mock::answers::greeting_grouping(), mock::answers::greeting_synthesis()]
}

fn started_event(root: &std::path::Path) -> Event {
    read_union(Layout::new(root))
        .expect("union")
        .into_iter()
        .find(|event| matches!(event.kind, EventKind::PlanExecuteStarted { .. }))
        .expect("plan.execute.started")
}

fn journal_kinds(root: &std::path::Path) -> Vec<String> {
    read_union(Layout::new(root))
        .expect("union")
        .into_iter()
        .filter_map(|event| {
            let wire = serde_json::to_value(&event).expect("serialize");
            wire.get("event").and_then(|v| v.as_str()).map(str::to_owned)
        })
        .collect()
}

fn live_plan_digest(root: &std::path::Path) -> String {
    let bytes = fs::read(Layout::new(root).plan_path()).expect("plan.yaml");
    format!("sha256:{}", sha256_hex(&bytes))
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
                gap_policy: GapPolicy::Strict,
            },
            discovery_digest: None,
        },
    );
    append_for(Layout::new(root), DEFAULT_WRITER, &[event]).expect("stamp epoch");
}

/// Acceptance #13 / D24 — drop one of two refined slices; gaps, Ready,
/// and the execute gap gate ignore the dropped entry while the sibling
/// remains on the plan (no second `plan remove`).
#[tokio::test]
async fn dropped_slice_excluded_from_gaps_ready_and_gap_gate() {
    let session = Session::bare(Vec::new());
    init_mock(&session).await;
    let root = session.root();

    write_refined(
        root,
        "gappy",
        r"requirements:
  - id: REQ-009
    title: contradiction left behind
    statement: ''
    status: conflict
    sources: [intent]
",
    );
    write_refined(
        root,
        "clean",
        r"requirements:
  - id: REQ-001
    title: login works
    statement: ''
    status: agreed
    sources: [intent]
",
    );
    let plan = plan_with_changes(vec![leaf("gappy"), leaf("clean")]);
    write_plan(root, &plan);

    let before = run::<Gaps, _, _>(session.provider(), GapsInput {}).await.expect("gaps");
    assert_eq!(before.rows.len(), 1);
    assert_eq!(before.rows[0].slice, "gappy");
    assert_eq!(before.rows[0].status, artifacts::spec::provenance::RequirementStatus::Conflict);

    let status_before =
        run::<Status, _, _>(session.provider(), StatusInput {}).await.expect("status");
    assert!(!status_before.ready, "open conflict keeps Ready false");

    // Stamp abandon membership; plan row stays for audit (D24).
    write_refined_meta(
        root,
        "gappy",
        r"requirements:
  - id: REQ-009
    title: contradiction left behind
    statement: ''
    status: conflict
    sources: [intent]
",
        true,
    );
    assert_eq!(
        Plan::load(&Layout::new(root).plan_path()).expect("reload").entries.len(),
        2,
        "drop excludes membership without plan remove"
    );

    let after = run::<Gaps, _, _>(session.provider(), GapsInput {}).await.expect("gaps after drop");
    assert!(after.rows.is_empty(), "dropped conflict excluded: {after:?}");

    let status = run::<Status, _, _>(session.provider(), StatusInput {}).await.expect("status");
    assert!(status.ready, "remaining clean sibling → Ready without plan remove");
    assert!(!serde_json::to_string(&status).expect("json").contains("approved"));

    // Covering epoch over the surviving leaf; gap gate must not see
    // the dropped conflict.
    let mut specs = BTreeMap::new();
    specs.insert(
        "clean".into(),
        LeafSpecCoverage::Existing {
            digest: dir_cid(&Layout::new(root).slice_dir("clean").join("specs"))
                .expect("specs cid")
                .to_string(),
        },
    );
    stamp_epoch(root, &live_plan_digest(root), specs);
    enforce_before_build(
        Layout::new(root),
        &plan,
        "clean",
        Timestamp::from_second(1_700_000_100).expect("timestamp"),
    )
    .expect("gap gate ignores dropped sibling conflict");
}

/// Acceptance #12 / D22 — open unknowns keep Ready false; deferring
/// then executing reaches Authorized without Ready; clearing unknowns
/// then projects Ready (deferrals never backfill Ready).
#[tokio::test]
async fn deferral_skips_ready_clearing_unknowns_reaches_ready() {
    let session = Session::bare(Vec::new());
    init_mock(&session).await;
    let root = session.root();

    write_refined(
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

    let open = run::<Status, _, _>(session.provider(), StatusInput {}).await.expect("status");
    assert!(!open.ready);
    assert!(!open.authorized);

    // Durable deferral, then execute; build may fail later without
    // pins — ignore the post-gate outcome and inspect milestones.
    run::<Defer, _, _>(
        session.provider(),
        DeferInput {
            selectors: vec![DeferSelector {
                slice: "a".into(),
                req: "REQ-003".into(),
            }],
            reason: Some("reset path deferred".into()),
            retract: false,
        },
    )
    .await
    .expect("plan defer");
    drop(run::<Execute, _, _>(session.provider(), ExecuteInput::default()).await);

    let deferred = run::<Status, _, _>(session.provider(), StatusInput {}).await.expect("status");
    assert!(deferred.authorized, "execute opens Authorized");
    assert!(!deferred.ready, "deferrals never contribute to Ready");
    assert!(!root.join(".emery/approvals").exists(), "no approvals/ tree");
    assert!(!serde_json::to_string(&deferred).expect("json").contains("approved"));

    // Clear the unknown on disk — Ready becomes true while the prior
    // epoch may still project Authorized (fresh execute is a separate act).
    write_refined(
        root,
        "a",
        r"requirements:
  - id: REQ-003
    title: reset path evidenced
    statement: ''
    status: agreed
    sources: [intent]
",
    );
    let cleared = run::<Status, _, _>(session.provider(), StatusInput {}).await.expect("status");
    assert!(cleared.ready, "clearing unknowns projects Ready before a clean execute");
}

/// Acceptance #8 — `plan.execute.started` wire payload matches the
/// closed `closed-plan` shape; covered-spec change is `plan-epoch-stale`.
#[tokio::test]
async fn coverage_wire_shape_and_stale_after_spec_change() {
    let session = Session::bare(suite_answers());
    let root = session.root().to_path_buf();
    init_mock(&session).await;

    run::<Author, _, _>(
        session.provider(),
        AuthorInput {
            name: "demo".to_string(),
            sources: support::greeting_binding(),
            intent: None,
            force: false,
        },
    )
    .await
    .expect("author");

    // Prefer shift-left: refine first so coverage stamps `existing`.
    support::refine(&session, "greeting").await.expect("refine");

    let specs_before =
        dir_cid(&root.join(".emery/slices/greeting/specs")).expect("specs").to_string();

    run::<Execute, _, _>(session.provider(), ExecuteInput::default())
        .await
        .expect("execute drains");

    let started = started_event(&root);
    let wire = serde_json::to_value(&started).expect("serialize started");
    assert_eq!(wire["event"], "plan.execute.started");
    let coverage = &wire["payload"]["coverage"];
    assert_eq!(coverage["kind"], "closed-plan");
    assert!(
        coverage["plan-digest"].as_str().is_some_and(|d| d.starts_with("sha256:")),
        "plan-digest: {coverage}"
    );
    assert_eq!(coverage["specs"]["greeting"]["kind"], "existing");
    assert_eq!(coverage["specs"]["greeting"]["digest"], specs_before);
    assert_eq!(coverage["gap-policy"], "strict", "effective policy on the wire: {coverage}");
    assert!(coverage.get("unknown-waivers").is_none(), "waiver field deleted: {coverage}");
    assert!(!wire.to_string().contains("approved"));
    assert!(!root.join(".emery/approvals").exists());

    // Covered-spec change against a live leaf → `plan-epoch-stale`
    // (execute drained greeting into the archive above).
    let session2 = Session::bare(Vec::new());
    init_mock(&session2).await;
    write_refined(
        session2.root(),
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
    write_plan(session2.root(), &plan);
    let digest =
        dir_cid(&session2.root().join(".emery/slices/a/specs")).expect("specs").to_string();
    let mut specs = BTreeMap::new();
    specs.insert(
        "a".into(),
        LeafSpecCoverage::Existing {
            digest: digest.clone(),
        },
    );
    stamp_epoch(session2.root(), &live_plan_digest(session2.root()), specs);
    fs::write(session2.root().join(".emery/slices/a/specs/extra.md"), "x\n").expect("drift");
    let err = enforce_before_build(
        Layout::new(session2.root()),
        &plan,
        "a",
        Timestamp::from_second(1_700_000_100).expect("timestamp"),
    )
    .expect_err("spec change → stale");
    assert_eq!(err.variant_str(), "plan-epoch-stale");
    assert_ne!(
        dir_cid(&session2.root().join(".emery/slices/a/specs")).expect("live").to_string(),
        digest
    );
}

/// Acceptance #14 / D26 — after a successful author (no refine yet),
/// author hint and fresh-plan status resume name `emery plan execute` /
/// `/emery:execute`; status may still suggest `slice refine`.
#[tokio::test]
async fn post_author_resume_names_execute() {
    let session = Session::bare(suite_answers());
    init_mock(&session).await;

    let authored = run::<Author, _, _>(
        session.provider(),
        AuthorInput {
            name: "demo".to_string(),
            sources: support::greeting_binding(),
            intent: None,
            force: false,
        },
    )
    .await
    .expect("author");

    assert!(
        authored.hint.contains("emery plan execute"),
        "author hint must name plan execute: {}",
        authored.hint
    );
    assert!(
        !authored.hint.contains("plan refine") && !authored.hint.contains("plan approve"),
        "hint must not invent plan refine/approve: {}",
        authored.hint
    );
    assert!(
        !session.root().join(".emery/slices/greeting/model.yaml").exists(),
        "author stays topology-only"
    );

    let status = run::<Status, _, _>(session.provider(), StatusInput {}).await.expect("status");
    assert_eq!(status.resume.as_deref(), Some("/emery:execute"));
    assert_eq!(status.next_action, "refine greeting");
    assert!(!status.ready);
    assert!(!status.authorized);
    let text = {
        let mut out = Vec::new();
        project::handler::Render::render(&status, &mut out).expect("render");
        String::from_utf8(out).expect("utf8")
    };
    assert!(!text.contains("approved"), "never project approved: {text}");
    assert!(!text.contains("plan refine"), "{text}");
}

/// Acceptance #15 / D9 — under execute, one-member wave opens before
/// build; merge projects wave-committed; postflight failure leaves the
/// merge accepted (non-rollback).
#[tokio::test]
async fn wave_opened_before_build_under_execute_postflight_keeps_commit() {
    let session = Session::bare(suite_answers());
    let root = session.root().to_path_buf();
    init_mock(&session).await;

    run::<Author, _, _>(
        session.provider(),
        AuthorInput {
            name: "demo".to_string(),
            sources: support::greeting_binding(),
            intent: None,
            force: false,
        },
    )
    .await
    .expect("author");

    // Happy wave ordering under execute (refine-under-epoch path).
    let drained = run::<Execute, _, _>(session.provider(), ExecuteInput::default())
        .await
        .expect("execute drains");
    assert_eq!(drained.status, "drained");
    let ran: Vec<(&str, LoopStep)> =
        drained.phases.iter().map(|phase| (phase.slice.as_str(), phase.step)).collect();
    assert_eq!(
        ran,
        [
            ("greeting", LoopStep::Refine),
            ("greeting", LoopStep::Build),
            ("greeting", LoopStep::Merge),
        ]
    );

    let kinds = journal_kinds(&root);
    let opened = kinds.iter().position(|k| k == "target.wave.opened").expect("wave.opened");
    let built = kinds.iter().position(|k| k == "slice.build.succeeded").expect("build.succeeded");
    let committed =
        kinds.iter().position(|k| k == "target.merge.wave-committed").expect("wave-committed");
    assert!(opened < built, "wave opens before build: {kinds:?}");
    assert!(built < committed, "commit after build: {kinds:?}");
    assert!(kinds.iter().any(|k| k == "target.merge.wave-succeeded"), "{kinds:?}");
    assert!(!root.join(".emery/slices/greeting/build/patch.yaml").exists());

    // Fresh tree: postflight failure after wave-committed is non-rollback.
    let session = Session::bare(suite_answers());
    let root = session.root().to_path_buf();
    init_mock(&session).await;
    run::<Author, _, _>(
        session.provider(),
        AuthorInput {
            name: "demo".to_string(),
            sources: support::greeting_binding(),
            intent: None,
            force: false,
        },
    )
    .await
    .expect("author");
    fs::write(root.join(behaviour::FAIL_MERGE_POSTFLIGHT_MARKER), "").expect("marker");

    let err = run::<Execute, _, _>(session.provider(), ExecuteInput::default())
        .await
        .expect_err("postflight fails");
    assert!(err.to_string().contains("merge-postflight-failed"), "{err}");

    let kinds = journal_kinds(&root);
    assert!(kinds.iter().any(|k| k == "target.wave.opened"), "{kinds:?}");
    assert!(kinds.iter().any(|k| k == "target.merge.wave-committed"), "{kinds:?}");
    assert!(kinds.iter().any(|k| k == "target.merge.wave-postflight-failed"), "{kinds:?}");
    assert!(!kinds.iter().any(|k| k == "target.merge.wave-succeeded"), "{kinds:?}");
    assert!(root.join(".emery/specs/greeting/spec.md").is_file(), "merge stands");
    assert!(!root.join(".emery/slices/greeting").exists(), "slice archived");
}
