//! RFC-86a Acceptance #5 / D4 — build-scope exclusion: the request's
//! `deferred[]`, the fail-fast deferred-covered gate under the RFC-90
//! phase machine, the record's consumed set, and disposition-drift
//! staleness at merge time.

mod support;

use std::fs;

use change::{LoopStep, plan};
use diagnostics::DiagnosticKind;
use mock::behaviour;
use mock::invoke::run;
use mock::session::Session;
use project::GapPolicy;
use project::build_record::BuildRecord;
use project::config::Layout;
use project::seam::wire::BuildRequest;

/// The minimal profile whose refine mints one `[unknown]` row
/// (`greeting/REQ-001`).
fn unknown_session() -> Session {
    Session::bare(vec![
        mock::answers::greeting_grouping(),
        mock::answers::greeting_unknown_synthesis(),
    ])
}

async fn scaffold(session: &Session) {
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
    run::<plan::handlers::Author, _, _>(
        session.provider(),
        plan::handlers::AuthorInput {
            name: "demo".to_string(),
            sources: support::greeting_binding(),
            intent: None,
            force: false,
        },
    )
    .await
    .expect("author");
}

async fn defer_req(session: &Session, retract: bool) {
    run::<plan::handlers::Defer, _, _>(
        session.provider(),
        plan::handlers::DeferInput {
            selectors: vec![plan::handlers::DeferSelector {
                slice: "greeting".into(),
                req: "REQ-001".into(),
            }],
            reason: (!retract).then(|| "carried to the next change".to_string()),
            retract,
        },
    )
    .await
    .expect("defer");
}

fn review_ids(body: &project::handler::ReportBody) -> Vec<String> {
    body.report()
        .findings
        .iter()
        .filter(|f| f.kind == DiagnosticKind::Review)
        .filter_map(|f| f.rule_id.clone())
        .collect()
}

async fn validate_reviews(session: &Session) -> Vec<String> {
    let body = run::<slice::handlers::Validate, _, _>(
        session.provider(),
        slice::handlers::ValidateInput {
            name: "greeting".to_string(),
        },
    )
    .await
    .expect("drift is review — validate still PASSes");
    review_ids(&body)
}

/// The request enumerates the deferred set (id, title, digest) and the
/// record binds the consumed digests; matching live dispositions read
/// as fresh.
#[tokio::test]
async fn request_and_record_bind_the_deferred_set() {
    let session = unknown_session();
    let root = session.root().to_path_buf();
    scaffold(&session).await;
    support::refine(&session, "greeting").await.expect("refine");
    defer_req(&session, false).await;

    support::build(&session, "greeting").await.expect("build under deferral");

    let layout = Layout::new(&root);
    let slice_dir = layout.slice_dir("greeting");
    let text = fs::read_to_string(slice_dir.join("build/request.yaml")).expect("request.yaml");
    let request: BuildRequest = serde_saphyr::from_str(&text).expect("request parses");
    assert_eq!(request.deferred.len(), 1, "{:?}", request.deferred);
    let deferred = &request.deferred[0];
    assert_eq!(deferred.id, "REQ-001");
    assert_eq!(deferred.title, "greeting error handling");
    assert!(
        deferred.requirement_digest.starts_with("sha256:"),
        "digest-bound exclusion: {}",
        deferred.requirement_digest
    );

    let record = BuildRecord::load_latest(&slice_dir).expect("build record");
    assert_eq!(
        record.deferred,
        std::slice::from_ref(&deferred.requirement_digest),
        "record binds the consumed deferred digest set"
    );
    assert!(
        !slice::dispositions_drifted(layout, &slice_dir, "greeting").expect("probe"),
        "matching live dispositions are fresh"
    );
    assert!(
        !validate_reviews(&session).await.iter().any(|id| id == "slice-disposition-drifted"),
        "no drift advisory while the sets agree"
    );
}

/// A build phase report claiming coverage of a deferred requirement
/// halts the machine fail-fast (`target-build-deferred-covered`):
/// verify never dispatches, the failed canonical projection carries
/// the engine finding, and no record lands.
#[tokio::test]
async fn covered_deferred_requirement_refuses_build() {
    let session = unknown_session();
    let root = session.root().to_path_buf();
    scaffold(&session).await;
    support::refine(&session, "greeting").await.expect("refine");
    defer_req(&session, false).await;

    fs::write(root.join(behaviour::CLAIM_COVERED_MARKER), "REQ-001").expect("marker");

    let err = support::build(&session, "greeting").await.expect_err("machine gate refuses");
    assert!(err.to_string().contains("target-build-deferred-covered"), "{err}");
    // Fail-fast placement (RFC-86a D4 under the RFC-90 machine): the
    // gate fires on the admitted build round, before a verify /
    // review dispatch is spent on an out-of-contract candidate.
    assert_eq!(
        support::phase_files(&root, "greeting", 1),
        ["01-build.yaml"],
        "the coverage claim halts the attempt before verify dispatches"
    );
    let report = support::canonical_report(&root, "greeting");
    assert_eq!(report.status, slice::BuildStatus::Failure);
    assert!(
        report
            .findings
            .iter()
            .any(|f| f.rule_id.as_deref() == Some("target-build-deferred-covered")),
        "engine-authored terminal finding present: {:?}",
        report.findings
    );
    assert!(
        !BuildRecord::present(&Layout::new(&root).slice_dir("greeting")),
        "a refused build mints no record"
    );
}

/// `covered[]` naming only non-deferred requirements passes the gate.
#[tokio::test]
async fn covered_claim_without_deferral_passes() {
    let session = unknown_session();
    let root = session.root().to_path_buf();
    scaffold(&session).await;
    support::refine(&session, "greeting").await.expect("refine");

    fs::write(root.join(behaviour::CLAIM_COVERED_MARKER), "REQ-001").expect("marker");

    // Nothing deferred — the claim is a free assertion, not a violation.
    support::build(&session, "greeting").await.expect("build with a non-deferred claim");
}

/// Retracting a deferral after build drifts the record: the probe
/// flips and validate carries the `slice-disposition-drifted` review.
#[tokio::test]
async fn retraction_after_build_drifts_dispositions() {
    let session = unknown_session();
    let root = session.root().to_path_buf();
    scaffold(&session).await;
    support::refine(&session, "greeting").await.expect("refine");
    defer_req(&session, false).await;
    support::build(&session, "greeting").await.expect("build under deferral");

    let layout = Layout::new(&root);
    let slice_dir = layout.slice_dir("greeting");
    assert!(!slice::dispositions_drifted(layout, &slice_dir, "greeting").expect("probe"));

    defer_req(&session, true).await;

    assert!(
        slice::dispositions_drifted(layout, &slice_dir, "greeting").expect("probe"),
        "retraction reopens the row — the built record is stale"
    );
    let body = run::<slice::handlers::Validate, _, _>(
        session.provider(),
        slice::handlers::ValidateInput {
            name: "greeting".to_string(),
        },
    )
    .await
    .expect("drift is review — validate still PASSes");
    let drift = body
        .report()
        .findings
        .iter()
        .find(|f| f.rule_id.as_deref() == Some("slice-disposition-drifted"))
        .expect("drift review finding");
    // The detail names the recorded and live digest sets, mirroring
    // pin drift's pinned-vs-live digests.
    let record = BuildRecord::load_latest(&slice_dir).expect("build record");
    assert!(
        drift.impact.contains(&record.deferred[0]),
        "detail names the recorded digest: {}",
        drift.impact
    );
    assert!(drift.impact.contains("[none]"), "detail names the (empty) live set: {}", drift.impact);
}

/// A failed re-build leaves the newest wave without a record (orphan
/// wave). The staleness probe must stay true — merge would refuse
/// `slice-build-record-missing` against the orphan wave — so the next
/// resume re-builds instead of wedging on the older record.
#[tokio::test]
async fn failed_rebuild_orphan_wave_stays_stale() {
    let session = unknown_session();
    let root = session.root().to_path_buf();
    scaffold(&session).await;

    // Park the loop between build and merge: the build succeeded and
    // its record consumes the policy-minted deferral.
    fs::write(root.join(behaviour::FAIL_MERGE_PREFLIGHT_MARKER), "").expect("marker");
    run::<plan::handlers::Execute, _, _>(
        session.provider(),
        plan::handlers::ExecuteInput {
            gap_policy: Some(GapPolicy::Defer),
        },
    )
    .await
    .expect_err("parked at merge preflight");
    fs::remove_file(root.join(behaviour::FAIL_MERGE_PREFLIGHT_MARKER)).expect("remove marker");

    // Retract the deferral (drift) and make the redirected re-build
    // fail: the loop opens a new wave, the build dies, no record
    // consumes it.
    defer_req(&session, true).await;
    fs::write(root.join(behaviour::FAIL_BUILD_MARKER), "").expect("marker");
    let stopped = run::<plan::handlers::Execute, _, _>(
        session.provider(),
        plan::handlers::ExecuteInput {
            gap_policy: Some(GapPolicy::Defer),
        },
    )
    .await
    .expect_err("redirected re-build fails");
    assert!(stopped.to_string().contains("build-failed"), "{stopped}");
    fs::remove_file(root.join(behaviour::FAIL_BUILD_MARKER)).expect("remove marker");

    // The older record still projects the slice as built, but the
    // newest wave is an orphan — the probe must read stale, and
    // validate must carry the orphan-wave advisory.
    let layout = Layout::new(&root);
    assert!(
        slice::dispositions_drifted(layout, &layout.slice_dir("greeting"), "greeting")
            .expect("probe"),
        "an orphan wave (failed re-build) keeps the slice stale"
    );
    assert!(
        validate_reviews(&session).await.iter().any(|id| id == "slice-wave-record-missing"),
        "orphan wave carries the review advisory"
    );

    // The resume heals: re-build under the current dispositions (the
    // gate already re-minted the deferral before the failed attempt),
    // then merge against the fresh wave's record.
    let drained = run::<plan::handlers::Execute, _, _>(
        session.provider(),
        plan::handlers::ExecuteInput {
            gap_policy: Some(GapPolicy::Defer),
        },
    )
    .await
    .expect("resume re-builds the orphan wave and drains");
    assert_eq!(drained.status, "drained");
    let steps: Vec<LoopStep> = drained.phases.iter().map(|phase| phase.step).collect();
    assert_eq!(
        steps,
        [LoopStep::Build, LoopStep::Merge],
        "the orphan wave re-builds before merge; got {steps:?}"
    );
}

/// A slice parked at merge with BOTH drifted pins and drifted
/// dispositions re-refines first: the disposition redirect lands on
/// the pin-drift check, so the re-build never runs under stale
/// `base.yaml` pins.
#[tokio::test]
async fn drifted_pins_and_dispositions_rerefine_before_rebuild() {
    // One extra synthesis answer feeds the forced re-refine.
    let session = Session::bare(vec![
        mock::answers::greeting_grouping(),
        mock::answers::greeting_unknown_synthesis(),
        mock::answers::greeting_unknown_synthesis(),
    ]);
    let root = session.root().to_path_buf();
    scaffold(&session).await;

    // Park the loop between build and merge under the defer policy.
    fs::write(root.join(behaviour::FAIL_MERGE_PREFLIGHT_MARKER), "").expect("marker");
    run::<plan::handlers::Execute, _, _>(
        session.provider(),
        plan::handlers::ExecuteInput {
            gap_policy: Some(GapPolicy::Defer),
        },
    )
    .await
    .expect_err("parked at merge preflight");
    fs::remove_file(root.join(behaviour::FAIL_MERGE_PREFLIGHT_MARKER)).expect("remove marker");

    // Drift both probes: retract the deferral (disposition drift) and
    // plant an orphan source pin (pin drift).
    defer_req(&session, true).await;
    let layout = Layout::new(&root);
    let slice_dir = layout.slice_dir("greeting");
    let mut base = slice::Base::load(&slice_dir).expect("base.yaml after build");
    base.sources.insert("gone".into(), project::plan::value_cid("orphan pin"));
    base.write(&slice_dir).expect("plant orphan pin");

    // The resume must re-refine before re-building: the disposition
    // redirect (Merge → Build) lands on the pin-drift check
    // (Build → Refine), so the fresh pins are frozen first.
    let drained = run::<plan::handlers::Execute, _, _>(
        session.provider(),
        plan::handlers::ExecuteInput {
            gap_policy: Some(GapPolicy::Defer),
        },
    )
    .await
    .expect("resume re-refines, re-builds, and drains");
    assert_eq!(drained.status, "drained");
    let steps: Vec<LoopStep> = drained.phases.iter().map(|phase| phase.step).collect();
    assert_eq!(
        steps,
        [LoopStep::Refine, LoopStep::Build, LoopStep::Merge],
        "stale pins re-refine before the disposition-drift re-build; got {steps:?}"
    );
}

/// Loop staleness: a retraction between build and merge sends the
/// slice back through the build gate — strict re-adjudicates the
/// reopened row, defer re-mints, re-builds, and drains.
#[tokio::test]
async fn execute_rebuilds_after_deferral_retraction() {
    let session = unknown_session();
    let root = session.root().to_path_buf();
    scaffold(&session).await;

    // Park the loop between build and merge.
    fs::write(root.join(behaviour::FAIL_MERGE_PREFLIGHT_MARKER), "").expect("marker");
    let stopped = run::<plan::handlers::Execute, _, _>(
        session.provider(),
        plan::handlers::ExecuteInput {
            gap_policy: Some(GapPolicy::Defer),
        },
    )
    .await
    .expect_err("parked at merge preflight");
    assert!(stopped.to_string().contains("target-merge-preflight-failed"), "{stopped}");
    fs::remove_file(root.join(behaviour::FAIL_MERGE_PREFLIGHT_MARKER)).expect("remove marker");

    // Retract the policy-minted deferral: the parked build is stale.
    defer_req(&session, true).await;
    let layout = Layout::new(&root);
    assert!(
        slice::dispositions_drifted(layout, &layout.slice_dir("greeting"), "greeting")
            .expect("probe"),
        "retraction drifts the parked build"
    );

    // A strict resume goes back through the build gate instead of
    // merging the stale record — the reopened row refuses the build.
    let err = run::<plan::handlers::Execute, _, _>(
        session.provider(),
        plan::handlers::ExecuteInput {
            gap_policy: Some(GapPolicy::Strict),
        },
    )
    .await
    .expect_err("strict resume re-adjudicates the reopened row");
    assert!(err.to_string().contains("plan-gaps-unresolved"), "{err}");

    // A defer resume re-mints the disposition, re-builds, and drains.
    let drained = run::<plan::handlers::Execute, _, _>(
        session.provider(),
        plan::handlers::ExecuteInput {
            gap_policy: Some(GapPolicy::Defer),
        },
    )
    .await
    .expect("defer resume rebuilds and drains");
    assert_eq!(drained.status, "drained");
    let steps: Vec<(&str, LoopStep)> =
        drained.phases.iter().map(|phase| (phase.slice.as_str(), phase.step)).collect();
    assert_eq!(
        steps,
        [("greeting", LoopStep::Build), ("greeting", LoopStep::Merge)],
        "stale build re-runs before merge; got {steps:?}"
    );
}
