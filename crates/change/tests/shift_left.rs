//! RFC-86 Acceptance #5–6 / D12 / D14 — shift-left preferred path and
//! `refine-under-epoch` execute path.
//!
//! Preferred: author (topology only) → refine phase → gaps → execute
//! (build/merge only, coverage `existing`). Under-epoch: execute opens
//! with `refine-under-epoch`, refines, gap-gates, then builds.

mod support;

use change::{LoopStep, plan};
use mock::invoke::run;
use mock::session::Session;
use project::config::Layout;
use project::journal::{EventKind, LeafSpecCoverage, read_union};
use project::plan::dir_cid;

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

/// Acceptance #5 preferred path: author does not refine; specs via a
/// prior refine phase; execute with existing digests runs build/merge
/// only.
#[tokio::test]
async fn shift_left_refine_then_execute_build_merge() {
    let session = Session::bare(suite_answers());
    let root = session.root().to_path_buf();
    scaffold_init(&session).await;

    let authored = author(&session).await;
    assert_eq!(authored.slices, ["greeting"]);
    assert!(authored.hint.contains("emery plan execute"), "{}", authored.hint);
    assert!(
        !authored.hint.contains("plan refine"),
        "resume must not invent plan refine: {}",
        authored.hint
    );

    // Author is topology-only — no extract / synthesis artifacts.
    let slice_dir = root.join(".emery/slices/greeting");
    assert!(!slice_dir.join("model.yaml").exists(), "author must not mint model.yaml");
    assert!(!slice_dir.join("spec.md").exists(), "author must not mint spec.md");
    assert!(!slice_dir.join("base.yaml").exists(), "author must not write base.yaml");
    let journal = journal_text(&root);
    assert!(
        !journal.contains("slice.transition.refined")
            && !journal.contains("slice.synthesize.")
            && !journal.contains("slice.extract.completed"),
        "author must not refine: {journal}"
    );

    support::refine(&session, "greeting").await.expect("refine phase mints specs");

    assert!(slice_dir.join("model.yaml").is_file(), "specs via the refine phase");
    assert!(slice_dir.join("base.yaml").is_file(), "pins at refine");

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
    let specs_digest = dir_cid(&slice_dir.join("specs")).expect("specs cid").to_string();

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
        "shift-left execute must not re-refine; got {ran:?}"
    );

    let project::journal::ClosedPlanCoverage::ClosedPlan { specs, .. } = started_coverage(&root);
    assert_eq!(
        specs.get("greeting"),
        Some(&LeafSpecCoverage::Existing { digest: specs_digest }),
        "preferred path stamps existing digests; got {specs:?}"
    );

    let journal = journal_text(&root);
    assert!(journal.contains("plan.execute.started"), "{journal}");
    assert!(journal.contains("target.wave.opened"), "{journal}");
    assert!(journal.contains("target.merge.wave-committed"), "{journal}");
    assert!(!root.join(".emery/slices/greeting/build/patch.yaml").exists());
}

/// Acceptance #5 under-epoch path: execute authorizes refine for
/// unspec'd leaves; refine → gap gate → build under that epoch.
#[tokio::test]
async fn refine_under_epoch_then_gap_gate_build() {
    let session = Session::bare(suite_answers());
    let root = session.root().to_path_buf();
    scaffold_init(&session).await;
    author(&session).await;

    assert!(
        !root.join(".emery/slices/greeting/model.yaml").exists(),
        "execute starts over unspec'd leaf"
    );

    let executed = run::<plan::handlers::Execute, _, _>(
        session.provider(),
        plan::handlers::ExecuteInput::default(),
    )
    .await
    .expect("under-epoch execute drains");
    assert_eq!(executed.status, "drained");
    let ran: Vec<(&str, LoopStep)> =
        executed.phases.iter().map(|phase| (phase.slice.as_str(), phase.step)).collect();
    assert_eq!(
        ran,
        [
            ("greeting", LoopStep::Refine),
            ("greeting", LoopStep::Build),
            ("greeting", LoopStep::Merge),
        ],
        "under-epoch: refine before build; got {ran:?}"
    );

    let project::journal::ClosedPlanCoverage::ClosedPlan { specs, .. } = started_coverage(&root);
    assert_eq!(
        specs.get("greeting"),
        Some(&LeafSpecCoverage::RefineUnderEpoch),
        "unspec'd at execute start → refine-under-epoch; got {specs:?}"
    );

    // Gap gate ran before build (clean specs pass); wave + merge prove
    // privileged work proceeded under the epoch.
    let journal = journal_text(&root);
    assert!(journal.contains("plan.execute.started"), "{journal}");
    assert!(
        journal.contains("slice.build.succeeded") || journal.contains("target.wave.opened"),
        "{journal}"
    );
    assert!(journal.contains("target.merge.wave-committed"), "{journal}");
    assert!(root.join(".emery/specs/greeting/spec.md").is_file());
}
