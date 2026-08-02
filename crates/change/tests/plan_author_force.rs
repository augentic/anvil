//! `plan author --force` replace policy at the public operation boundary.

mod support;

use change::plan;
use mock::invoke::run;
use mock::session::Session;
use project::plan::{Plan, Status};

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

#[tokio::test]
async fn existing_refused_without_force() {
    let session = Session::scripted("mock", vec![mock::answers::greeting_grouping()]);
    init(&session).await;
    author(&session, false).await.expect("first author");

    let err = author(&session, false).await.expect_err("second author refuses");
    let detail = err.to_string();
    assert!(detail.contains("plan-already-exists"), "{detail}");
}

#[tokio::test]
async fn force_replaces_replaceable() {
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

    let replaced = author(&session, true).await.expect("force re-authors");
    assert_eq!(replaced.slices, ["greeting"]);

    let after = Plan::load(&plan_path).expect("plan after force");
    assert_eq!(after.entries.len(), 1);
    assert!(after.entries.iter().all(|entry| entry.status == Status::Pending));
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

    // Walk an entry forward so the plan is no longer replaceable —
    // `--force` still recreates it unconditionally.
    let plan_path = session.root().join("plan.yaml");
    let mut plan = Plan::load(&plan_path).expect("load");
    plan.entries[0].status = Status::Done;
    plan.save(&plan_path).expect("mark entry done");

    let replaced = author(&session, true).await.expect("force re-authors progressed plan");
    assert_eq!(replaced.slices, ["greeting"]);

    let after = Plan::load(&plan_path).expect("plan after force");
    assert_eq!(after.entries.len(), 1);
    assert!(after.entries.iter().all(|entry| entry.status == Status::Pending));
}
