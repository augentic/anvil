//! RFC-91 D1/D5 — the staged workflow: author (topology only) →
//! `plan refine` (the specification drain) → gaps → execute
//! (build/merge only, coverage over exact refinement digests).
//! Execute over an unrefined leaf fails typed
//! `plan-refinement-required` and never auto-refines.

mod support;

use change::{LoopStep, plan};
use mock::invoke::run;
use mock::session::Session;
use project::config::Layout;
use project::journal::{EventKind, read_union};

fn suite_answers() -> Vec<String> {
    vec![mock::answers::greeting_grouping(), mock::answers::greeting_synthesis()]
}

fn journal_text(root: &std::path::Path) -> String {
    let events = read_union(Layout::new(root)).expect("journal union");
    events
        .iter()
        .map(|event| serde_json::to_string(event).expect("serialize"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn started_coverage(root: &std::path::Path) -> project::journal::ClosedPlanCoverage {
    read_union(Layout::new(root))
        .expect("union")
        .into_iter()
        .find_map(|event| match event.kind {
            EventKind::PlanExecuteStarted { coverage, .. } => Some(coverage),
            _ => None,
        })
        .expect("plan.execute.started")
}

async fn scaffold_init(session: &Session) {
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

async fn author(session: &Session) -> plan::handlers::AuthorBody {
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
    .expect("author")
}

/// The staged path: author does not refine; specs and the refinement
/// manifest arrive through the `plan refine` drain; execute covers the
/// exact refinement digest and runs build/merge only.
#[tokio::test]
async fn shift_left_refine_execute() {
    let session = Session::bare(suite_answers());
    let root = session.root().to_path_buf();
    scaffold_init(&session).await;

    let authored = author(&session).await;
    assert_eq!(authored.slices, ["greeting"]);
    assert!(
        authored.hint.contains("emery plan refine"),
        "author exits pointing at the refinement drain: {}",
        authored.hint
    );

    // Author is topology-only — no extract / synthesis artifacts.
    let slice_dir = root.join(".emery/slices/greeting");
    assert!(!slice_dir.join("model.yaml").exists(), "author must not mint model.yaml");
    assert!(!slice_dir.join("spec.md").exists(), "author must not mint spec.md");
    assert!(!slice_dir.join("refinement.yaml").exists(), "author must not write the manifest");
    assert!(!slice_dir.join("base.yaml").exists(), "base.yaml is deleted (RFC-91)");
    let journal = journal_text(&root);
    assert!(
        !journal.contains("slice.transition.refined")
            && !journal.contains("slice.synthesize.")
            && !journal.contains("slice.extract.completed"),
        "author must not refine: {journal}"
    );

    let refined = support::refine_plan(&session).await;
    assert_eq!(refined.refined, ["greeting"]);

    assert!(slice_dir.join("model.yaml").is_file(), "specs via the refine drain");
    assert!(slice_dir.join("refinement.yaml").is_file(), "manifest at refine");
    assert!(!slice_dir.join("base.yaml").exists(), "no base.yaml pin survives RFC-91");

    let gaps = run::<plan::handlers::Gaps, _, _>(session.provider(), plan::handlers::GapsInput {})
        .await
        .expect("gaps");
    assert!(gaps.rows.is_empty(), "clean greeting → no typed gaps: {gaps:?}");

    let status =
        run::<plan::handlers::Status, _, _>(session.provider(), plan::handlers::StatusInput {})
            .await
            .expect("status");
    assert!(status.ready, "refined + clean gaps → Ready");
    assert!(!status.authorized, "no epoch until execute");

    // Capture before execute — merge archives the slice tree.
    let refinement_digest = support::manifest_digest(&root, "greeting");

    let executed = run::<plan::handlers::Execute, _, _>(
        session.provider(),
        plan::handlers::ExecuteInput::default(),
    )
    .await
    .expect("execute drains build/merge");
    assert_eq!(executed.status, "drained");
    let ran: Vec<(&str, LoopStep)> =
        executed.phases.iter().map(|phase| (phase.slice.as_str(), phase.step)).collect();
    assert_eq!(
        ran,
        [("greeting", LoopStep::Build), ("greeting", LoopStep::Merge)],
        "execute must not re-refine; got {ran:?}"
    );

    let project::journal::ClosedPlanCoverage::ClosedPlan { refinements, .. } =
        started_coverage(&root);
    assert_eq!(
        refinements.get("greeting"),
        Some(&refinement_digest),
        "coverage stamps the exact refinement digest; got {refinements:?}"
    );

    let journal = journal_text(&root);
    assert!(journal.contains("plan.execute.started"), "{journal}");
    assert!(journal.contains("target.wave.opened"), "{journal}");
    assert!(journal.contains("target.merge.wave-committed"), "{journal}");
    assert!(!root.join(".emery/slices/greeting/build/patch.yaml").exists());
}

/// Execute over an unrefined leaf fails typed
/// `plan-refinement-required` before any epoch, workspace, or wave —
/// execute never refines (RFC-91 D5).
#[tokio::test]
async fn unrefined_execute_fails() {
    let session = Session::bare(suite_answers());
    let root = session.root().to_path_buf();
    scaffold_init(&session).await;
    author(&session).await;

    assert!(
        !root.join(".emery/slices/greeting/refinement.yaml").exists(),
        "execute starts over an unrefined leaf"
    );

    let err = run::<plan::handlers::Execute, _, _>(
        session.provider(),
        plan::handlers::ExecuteInput::default(),
    )
    .await
    .expect_err("execute refuses the unrefined leaf");
    let detail = err.to_string();
    assert!(detail.contains("plan-refinement-required"), "{detail}");
    assert!(detail.contains("emery plan refine"), "points at the drain: {detail}");

    // Nothing privileged happened: no epoch, no refinement, no wave.
    let journal = journal_text(&root);
    assert!(!journal.contains("plan.execute.started"), "no epoch: {journal}");
    assert!(!journal.contains("slice.synthesize."), "execute never refines: {journal}");
    assert!(!journal.contains("target.wave.opened"), "no wave: {journal}");
    assert!(!root.join(".emery/slices/greeting/model.yaml").exists());

    // The drain then execute is the recovery path.
    support::refine_plan(&session).await;
    let executed = run::<plan::handlers::Execute, _, _>(
        session.provider(),
        plan::handlers::ExecuteInput::default(),
    )
    .await
    .expect("refined plan drains");
    assert_eq!(executed.status, "drained");
}
