//! `plan author --force` replace policy at the public operation boundary.

mod support;

use change::plan;
use jiff::Timestamp;
use mock::invoke::run;
use mock::session::Session;
use project::config::Layout;
use project::journal::{self, Event, EventKind};
use project::plan::{Plan, Status, collect_events, project_ladders};

async fn author(
    session: &Session, force: bool,
) -> Result<plan::handlers::AuthorBody, project::handler::Error> {
    run::<plan::handlers::Author, _, _>(
        session.provider(),
        plan::handlers::AuthorInput {
            name: "demo".to_string(),
            sources: support::greeting_binding(),
            intent: None,
            force,
        },
    )
    .await
}

async fn init(session: &Session) {
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

fn assert_projected_pending(root: &std::path::Path) {
    let plan = Plan::load(&Layout::new(root).plan_path()).expect("plan");
    let events = collect_events(Layout::new(root)).expect("events");
    let ladders = project_ladders(&plan, &events);
    // Force-author scaffolds an empty plan then re-projects; leftover
    // facts for prior slice names do not attach to the new entry set
    // when names differ, and when names match a prior archive would
    // project done — assert the fresh plan rows are pending when no
    // matching done fact exists for the current plan name+entries.
    assert_eq!(plan.entries.len(), 1);
    let yaml = std::fs::read_to_string(root.join("plan.yaml")).expect("plan.yaml");
    assert!(!yaml.contains("status:"), "no stored status field: {yaml}");
    assert!(
        ladders.values().all(|status| *status == Status::Pending)
            || ladders.is_empty() && plan.entries.is_empty(),
        "ladders={ladders:?}"
    );
}

#[tokio::test]
async fn existing_refused_without() {
    let session = Session::scripted("mock", vec![mock::answers::greeting_grouping()]);
    init(&session).await;
    author(&session, false).await.expect("first author");

    let err = author(&session, false).await.expect_err("second author refuses");
    let detail = err.to_string();
    assert!(detail.contains("plan-already-exists"), "{detail}");
}

#[tokio::test]
async fn replaces_replaceable() {
    // Two author runs → two reconcile answers.
    let session = Session::scripted(
        "mock",
        vec![mock::answers::greeting_grouping(), mock::answers::greeting_grouping()],
    );
    init(&session).await;
    author(&session, false).await.expect("first author");

    let plan_path = session.root().join("plan.yaml");
    let before = Plan::load(&plan_path).expect("plan after first author");
    assert!(!before.entries.is_empty(), "first author wrote slices");
    let main = before.sources.get("main").expect("main source");
    let first_cid = main.cid.clone().expect("main cid after author");
    assert_eq!(first_cid, project::plan::value_cid("The greeting service."));

    let replaced = author(&session, true).await.expect("force re-authors");
    assert_eq!(replaced.slices, ["greeting"]);

    let after = Plan::load(&plan_path).expect("plan after force");
    assert_eq!(
        after.sources.get("main").and_then(|b| b.cid.as_ref()),
        Some(&first_cid),
        "force re-author re-pins the same value to the same cid"
    );

    assert_projected_pending(session.root());
}

#[tokio::test]
async fn force_replaces_progressed() {
    // Two author runs → two reconcile answers.
    let session = Session::scripted(
        "mock",
        vec![mock::answers::greeting_grouping(), mock::answers::greeting_grouping()],
    );
    init(&session).await;
    author(&session, false).await.expect("first author");

    // Walk progress forward via facts so the plan is no longer
    // replaceable under projected ladders — `--force` still recreates
    // it unconditionally (scaffold wipes `plan.yaml` first).
    let plan_path = session.root().join("plan.yaml");
    let plan = Plan::load(&plan_path).expect("load");
    let slice = plan.entries[0].name.clone();
    let now = Timestamp::from_second(1_700_000_000).expect("timestamp");
    journal::append_one(
        Layout::new(session.root()),
        &Event::new(
            now,
            EventKind::SliceArchiveCreated {
                slice_name: slice,
                touched_specs: vec!["greeting".into()],
                outcome_summary: "merged".into(),
                merge_sha: None,
                decisions: Vec::new(),
            },
        ),
    )
    .expect("done fact");

    let replaced = author(&session, true).await.expect("force re-authors progressed plan");
    assert_eq!(replaced.slices, ["greeting"]);

    // Fresh scaffold + propose; the prior archive fact still names the
    // same slice, so projected ladder may show done — the force gate is
    // that author succeeded. Assert the on-disk plan has no status field.
    let yaml = std::fs::read_to_string(session.root().join("plan.yaml")).expect("plan.yaml");
    assert!(!yaml.contains("status:"), "no stored status field: {yaml}");
    let after = Plan::load(&plan_path).expect("plan after force");
    assert_eq!(after.entries.len(), 1);
}
