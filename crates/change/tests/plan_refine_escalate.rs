//! Refinement boundary-escalation: an inert amendment proposal, live
//! planning artifacts unchanged, and parked re-entry (RFC-88 D3).

mod support;

use change::plan;
use mock::definition::{Spec, mint};
use mock::invoke::run;
use mock::session::Session;
use project::config::Layout;
use project::journal::{Event, EventKind, ParkReason, append_one};
use project::plan::Proposal;
use project::slice::{LifecycleStatus, SliceMetadata};

fn seed_target(root: &std::path::Path, name: &str) -> std::path::PathBuf {
    let target = root.join(name);
    std::fs::create_dir_all(target.join(".emery")).expect("target .emery");
    std::fs::write(
        target.join(".emery/project.yaml"),
        format!("name: {name}\nadapter: omnia\nrules: {{}}\n"),
    )
    .expect("project.yaml");
    target
}

fn rebind_linked_mock(layout: Layout<'_>) {
    // Author records first-party pins (`intent@1.0.0`, `omnia@1.0.0`)
    // the native mock catalog cannot ensure. Rebind onto the linked
    // mock identity so extract and guidance can dispatch.
    let pin = project::adapter::catalog::Pin::parse("emery:mock@0.0.0").expect("mock pin");
    let mut plan = project::plan::Plan::load(&layout.plan_path()).expect("plan");
    for binding in plan.sources.values_mut() {
        binding.adapter = pin.clone();
    }
    for row in plan.targets.values_mut() {
        row.adapter = pin.clone();
    }
    plan.save(&layout.plan_path()).expect("rebind");
}

async fn author_greeting(session: &Session) {
    let target = seed_target(session.root(), "target-app");
    let definition = session.root().join("definition");
    let mut spec = Spec::degenerate("Ship the greeting.");
    spec.targets[0].locator = target.display().to_string();
    mint(&definition, &spec).expect("mint");
    run::<plan::handlers::Author, _, _>(
        session.provider(),
        plan::handlers::AuthorInput {
            name: "demo".to_string(),
            from: definition,
            wave: spec.wave,
            force: false,
        },
    )
    .await
    .expect("author");
    rebind_linked_mock(Layout::new(session.root()));
}

async fn status(session: &Session) -> project::plan::StatusBody {
    run::<plan::handlers::Status, _, _>(session.provider(), plan::handlers::StatusInput {})
        .await
        .expect("plan status")
}

fn manifest_path(root: &std::path::Path, slice: &str) -> std::path::PathBuf {
    root.join(".emery/change/slices").join(slice).join("refinement.yaml")
}

// Scripted escalation writes one inert proposal, leaves planning
// artifacts byte-identical, skips the `refined` transition, and parks
// `plan refine` re-entry without a further model dispatch. Value-backed
// intent focus adds no child leads to the candidate catalog.
#[tokio::test]
async fn boundary_escalation_parks() {
    let mut answers = mock::answers::greeting_author();
    answers.push(mock::answers::greeting_escalation());
    answers.push(mock::answers::greeting_leaf());
    let session = Session::scripted("mock", answers);
    author_greeting(&session).await;

    let layout = Layout::new(session.root());
    let leads = std::fs::read(layout.leads_path()).expect("leads.md");
    let decomp = std::fs::read(layout.decomposition_path()).expect("decomposition.yaml");
    let plan_bytes = std::fs::read(layout.plan_path()).expect("plan.yaml");

    let stopped = support::refine_slices(&session, &[])
        .await
        .expect_err("escalation parks the drain")
        .to_string();
    assert!(stopped.contains("plan-refine-stopped"), "{stopped}");
    assert!(stopped.contains("boundary-escalation"), "{stopped}");

    assert_eq!(std::fs::read(layout.leads_path()).expect("leads.md"), leads);
    assert_eq!(std::fs::read(layout.decomposition_path()).expect("decomposition.yaml"), decomp);
    assert_eq!(std::fs::read(layout.plan_path()).expect("plan.yaml"), plan_bytes);
    assert!(!manifest_path(session.root(), "greeting").exists(), "no refinement.yaml");
    assert!(
        !layout.slice_dir("greeting").join("model.yaml").exists(),
        "synthesis artifacts were not promoted"
    );

    let slice_dir = layout.slice_dir("greeting");
    let metadata = SliceMetadata::load(&slice_dir).expect("slice directory exists after extract");
    assert_eq!(
        LifecycleStatus::project(&slice_dir, &metadata),
        LifecycleStatus::Refining,
        "no refined transition"
    );

    let proposals = Proposal::load_all(layout).expect("proposals");
    assert_eq!(proposals.len(), 1, "one inert proposal");
    let Proposal::Boundary(boundary) = &proposals[0].1 else {
        panic!("expected boundary proposal");
    };
    assert_eq!(boundary.failed_leaf.as_str(), "greeting");
    assert_eq!(boundary.affected.len(), 1);
    assert_eq!(boundary.affected[0].source, "intent");
    assert_eq!(boundary.affected[0].lead, "intent");
    let pairs: Vec<(&str, &str)> = boundary
        .candidate_leads
        .iter()
        .map(|lead| (lead.source.as_str(), lead.lead.as_str()))
        .collect();
    assert_eq!(pairs, [("intent", "intent")], "value-backed intent skip adds no children");

    let body = status(&session).await;
    assert_eq!(body.next_action, "stop boundary-escalation");
    assert_eq!(body.slice.as_deref(), Some("greeting"));
    assert_eq!(body.current_step, Some(change::LoopStep::Refine));
    assert!(
        body.resume.as_deref().is_some_and(|cmd| cmd.starts_with("emery plan amend --proposal ")),
        "resume {}",
        body.resume.as_deref().unwrap_or("-")
    );

    session.model().assert_exhausted();
    let parked =
        support::refine_slices(&session, &[]).await.expect_err("re-entry stays parked").to_string();
    assert!(parked.contains("plan-refine-stopped"), "{parked}");
    assert!(parked.contains("already parks"), "{parked}");
    session.model().assert_exhausted();
}

// A planted budget-park fact projects `stop refine-budget-exhausted`
// with refine as the resume path. `MAX_JUDGMENTS` is 128, so a live
// exhaust loop is not a practical native fixture.
#[tokio::test]
async fn budget_park_status() {
    let session = Session::scripted("mock", mock::answers::greeting_author());
    author_greeting(&session).await;

    let layout = Layout::new(session.root());
    append_one(
        layout,
        &Event::new(
            jiff::Timestamp::now(),
            EventKind::SliceRefinementParked {
                slice_name: "greeting".into(),
                reason: ParkReason::BudgetExhausted,
                proposal: None,
            },
        ),
    )
    .expect("plant budget park");

    let body = status(&session).await;
    assert_eq!(body.next_action, "stop refine-budget-exhausted");
    assert_eq!(body.slice.as_deref(), Some("greeting"));
    assert_eq!(body.current_step, Some(change::LoopStep::Refine));
    assert_eq!(body.resume.as_deref(), Some("emery plan refine"));
}
