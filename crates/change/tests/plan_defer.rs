//! Integration coverage for `emery plan defer` / `--retract`
//! (RFC-86a D3): durable operator deferral facts, retraction, and the
//! `plan-deferral-invalid` refusal arms.

mod support;

use change::plan::handlers::{
    Defer, DeferAction, DeferBody, DeferInput, DeferSelector, Gaps, GapsInput,
};
use change::{GapsBody, Plan};
use mock::invoke::run;
use mock::session::Session;
use project::config::Layout;
use project::journal::{DeferralOrigin, EventKind, read_union};
use project::plan::Disposition;
use project::slice::RequirementBody;
use support::{change as entry, plan_with_changes};

fn write_plan(project: &Session, plan: &Plan) {
    let yaml = serde_saphyr::to_string(plan).expect("serialize plan");
    std::fs::write(project.root().join("plan.yaml"), yaml).expect("write plan.yaml");
}

fn write_slice_model(root: &std::path::Path, name: &str, model: &str) {
    let slice_dir = root.join(".emery").join("slices").join(name);
    std::fs::create_dir_all(&slice_dir).expect("slice dir");
    std::fs::write(slice_dir.join("metadata.yaml"), "target: demo-target@1.0.0\n")
        .expect("metadata");
    std::fs::write(slice_dir.join("model.yaml"), model).expect("model");
}

/// Canonical digest of a title-only body — the shape the fixture
/// models in this suite carry (no statement/scenarios/notes).
fn title_digest(title: &str) -> String {
    RequirementBody {
        title,
        statement: "",
        scenarios: &[],
        notes: None,
    }
    .digest()
}

/// One slice `auth-login` with an `[unknown]`, a `[conflict]`, and a
/// `[divergence]` row.
fn fixture() -> Session {
    let project = Session::scripted("demo", Vec::new());
    let plan = plan_with_changes(vec![entry("auth-login")]);
    write_plan(&project, &plan);
    write_slice_model(
        project.root(),
        "auth-login",
        r"requirements:
  - id: REQ-001
    title: reset path not evidenced
    status: unknown
  - id: REQ-002
    title: session TTL tied
    status: conflict
  - id: REQ-003
    title: retry budget divergence
    status: divergence
",
    );
    project
}

fn selector(slice: &str, req: &str) -> DeferSelector {
    DeferSelector {
        slice: slice.into(),
        req: req.into(),
    }
}

fn defer_input(selectors: Vec<DeferSelector>, reason: Option<&str>, retract: bool) -> DeferInput {
    DeferInput {
        selectors,
        reason: reason.map(Into::into),
        retract,
    }
}

async fn gaps(project: &Session) -> GapsBody {
    run::<Gaps, _, _>(project.provider(), GapsInput {}).await.expect("plan gaps")
}

/// Unwrap a validation failure into its kebab code + detail.
fn validation(err: &project::handler::Error) -> (String, String) {
    match err.core() {
        error::Error::Validation { code, detail } => (code.to_string(), detail.clone()),
        other => panic!("expected Validation, got {other:?}"),
    }
}

#[tokio::test]
async fn defer_writes_operator_facts_covering_unknown_and_conflict() {
    let project = fixture();

    let body: DeferBody = run::<Defer, _, _>(
        project.provider(),
        defer_input(
            vec![selector("auth-login", "REQ-001"), selector("auth-login", "REQ-002")],
            Some("reset path deferred to next change"),
            false,
        ),
    )
    .await
    .expect("plan defer");

    assert_eq!(body.action, DeferAction::Deferred);
    assert_eq!(body.reason, "reset path deferred to next change");
    assert_eq!(body.gaps.len(), 2);
    assert_eq!(body.gaps[0].req, "REQ-001");
    assert_eq!(body.gaps[0].requirement_digest, title_digest("reset path not evidenced"));
    assert_eq!(body.gaps[1].req, "REQ-002");
    assert_eq!(body.gaps[1].requirement_digest, title_digest("session TTL tied"));

    // Fact content: one `gap.deferred` per selector, origin operator,
    // digest bound to the live row's body.
    let events = read_union(Layout::new(project.root())).expect("union");
    let facts: Vec<_> = events
        .iter()
        .filter_map(|event| match &event.kind {
            EventKind::GapDeferred {
                slice,
                req,
                requirement_digest,
                reason,
                origin,
            } => Some((slice.as_str(), req.as_str(), requirement_digest, reason, origin)),
            _ => None,
        })
        .collect();
    assert_eq!(facts.len(), 2);
    for (slice, _, _, reason, origin) in &facts {
        assert_eq!(*slice, "auth-login");
        assert_eq!(reason.as_str(), "reset path deferred to next change");
        assert_eq!(**origin, DeferralOrigin::Operator);
    }
    assert_eq!(facts[0].1, "REQ-001");
    assert_eq!(*facts[0].2, title_digest("reset path not evidenced"));
    assert_eq!(facts[1].1, "REQ-002");
    assert_eq!(*facts[1].2, title_digest("session TTL tied"));

    // Durability at projection level: nothing was re-supplied, yet a
    // fresh projection (the shape every later epoch joins) still
    // covers both rows; divergence keeps no disposition.
    let inventory = gaps(&project).await;
    assert_eq!(inventory.rows[0].disposition, Some(Disposition::Deferred));
    assert_eq!(inventory.rows[1].disposition, Some(Disposition::Deferred));
    assert_eq!(inventory.rows[2].disposition, None);
}

#[tokio::test]
async fn retract_reopens_a_live_deferral() {
    let project = fixture();
    let target = vec![selector("auth-login", "REQ-001")];

    run::<Defer, _, _>(project.provider(), defer_input(target.clone(), Some("waits"), false))
        .await
        .expect("defer");
    assert_eq!(gaps(&project).await.rows[0].disposition, Some(Disposition::Deferred));

    // Retract without `--reason`: the synthesized reason is recorded.
    let body: DeferBody = run::<Defer, _, _>(project.provider(), defer_input(target, None, true))
        .await
        .expect("retract");
    assert_eq!(body.action, DeferAction::Retracted);
    assert_eq!(body.reason, "retracted by operator");

    let events = read_union(Layout::new(project.root())).expect("union");
    let retraction = events
        .iter()
        .find_map(|event| match &event.kind {
            EventKind::GapDeferralRetracted { reason, origin, .. } => Some((reason, origin)),
            _ => None,
        })
        .expect("retraction fact");
    assert_eq!(retraction.0, "retracted by operator");
    assert_eq!(*retraction.1, DeferralOrigin::Operator);

    assert_eq!(gaps(&project).await.rows[0].disposition, Some(Disposition::Open));
}

#[tokio::test]
async fn unknown_selector_refused() {
    let project = fixture();
    let err = run::<Defer, _, _>(
        project.provider(),
        defer_input(vec![selector("auth-login", "REQ-404")], Some("why"), false),
    )
    .await
    .expect_err("unknown selector");
    let (code, detail) = validation(&err);
    assert_eq!(code, "plan-deferral-invalid");
    assert!(detail.contains("auth-login/REQ-404"), "{detail}");
}

#[tokio::test]
async fn missing_reason_on_defer_refused() {
    let project = fixture();
    let err = run::<Defer, _, _>(
        project.provider(),
        defer_input(vec![selector("auth-login", "REQ-001")], None, false),
    )
    .await
    .expect_err("missing reason");
    let (code, detail) = validation(&err);
    assert_eq!(code, "plan-deferral-invalid");
    assert!(detail.contains("--reason"), "{detail}");

    // Whitespace-only counts as missing.
    let err = run::<Defer, _, _>(
        project.provider(),
        defer_input(vec![selector("auth-login", "REQ-001")], Some("   "), false),
    )
    .await
    .expect_err("blank reason");
    assert_eq!(validation(&err).0, "plan-deferral-invalid");
}

#[tokio::test]
async fn divergence_row_takes_no_disposition() {
    let project = fixture();
    let err = run::<Defer, _, _>(
        project.provider(),
        defer_input(vec![selector("auth-login", "REQ-003")], Some("why"), false),
    )
    .await
    .expect_err("divergence refused");
    let (code, detail) = validation(&err);
    assert_eq!(code, "plan-deferral-invalid");
    assert!(detail.contains("divergence"), "{detail}");
}

#[tokio::test]
async fn retract_of_non_live_deferral_refused() {
    let project = fixture();
    let err = run::<Defer, _, _>(
        project.provider(),
        defer_input(vec![selector("auth-login", "REQ-001")], None, true),
    )
    .await
    .expect_err("nothing to retract");
    let (code, detail) = validation(&err);
    assert_eq!(code, "plan-deferral-invalid");
    assert!(detail.contains("no live deferral"), "{detail}");
}

#[tokio::test]
async fn bad_selector_in_batch_writes_no_facts() {
    let project = fixture();
    let err = run::<Defer, _, _>(
        project.provider(),
        defer_input(
            vec![selector("auth-login", "REQ-001"), selector("auth-login", "REQ-404")],
            Some("why"),
            false,
        ),
    )
    .await
    .expect_err("batch with a bad selector");
    assert_eq!(validation(&err).0, "plan-deferral-invalid");

    let events = read_union(Layout::new(project.root())).expect("union");
    assert!(
        !events.iter().any(|event| matches!(event.kind, EventKind::GapDeferred { .. })),
        "a refused batch appends nothing"
    );
    assert_eq!(gaps(&project).await.rows[0].disposition, Some(Disposition::Open));
}
