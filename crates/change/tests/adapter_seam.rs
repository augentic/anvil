//! Typed adapter failures crossing the seam: a parked `plan refine`
//! drain, a parked build or merge in the execute loop, or the
//! outputs-exist gate — with the adapter's typed detail preserved.
//! Survey-during-author coverage is retired with the old author path.

mod support;

use change::plan;
use mock::invoke::run;
use mock::session::Session;
use project::adapter::catalog::Pin;
use project::plan::SourceBinding;

async fn execute_err(session: &Session) -> String {
    run::<plan::handlers::Execute, _, _>(
        session.provider(),
        plan::handlers::ExecuteInput::default(),
    )
    .await
    .expect_err("the failing phase parks the loop")
    .to_string()
}

async fn refine_err(session: &Session) -> String {
    support::refine_slices(session, &[])
        .await
        .expect_err("the failing refinement parks the drain")
        .to_string()
}

#[tokio::test]
async fn survey_ensures_pinned() {
    let session = Session::scripted("mock", Vec::new());

    let mut bindings = std::collections::BTreeMap::new();
    bindings.insert(
        "main".to_string(),
        SourceBinding::intent(
            Pin::emery("mock", semver::Version::new(1, 0, 0)),
            "The greeting service.",
        ),
    );
    let plan_path = session.root().join(".emery/change/plan.yaml");
    project::plan::scaffold(&plan_path, "demo", bindings, false)
        .expect("scaffold")
        .save(&plan_path)
        .expect("save plan.yaml");

    let err = run::<change::source::Survey, _, _>(
        session.provider(),
        change::source::SurveyInput {
            source: "main".to_string(),
            plan: None,
        },
    )
    .await
    .expect_err("ensure refuses the pin before dispatch");
    let detail = err.to_string();
    assert!(detail.contains("adapter-not-linked"), "{detail}");
    assert!(detail.contains("emery:mock@1.0.0"), "{detail}");
}

#[tokio::test]
async fn extract_failure_parks() {
    let session = Session::scripted("mock", Vec::new());
    support::write_plan_fixture(
        session.root(),
        "demo",
        &[("main", "mock-fail-extract", "The greeting service.")],
        &[("greeting", "main", "greeting")],
    );
    let detail = refine_err(&session).await;
    assert!(detail.contains("plan-refine-stopped"), "{detail}");
    assert!(detail.contains("mock extract failure"), "typed detail preserved: {detail}");
}

#[tokio::test]
async fn guidance_failure_parks() {
    let session = Session::scripted("mock-fail-guidance", Vec::new());
    support::write_greeting_plan(session.root());
    let detail = refine_err(&session).await;
    assert!(detail.contains("plan-refine-stopped"), "{detail}");
    assert!(detail.contains("mock guidance failure"), "typed detail preserved: {detail}");
}

#[tokio::test]
async fn build_failure_parks() {
    let session = Session::scripted("mock-fail-build", vec![mock::answers::greeting_synthesis()]);
    support::write_greeting_plan(session.root());
    support::refine_plan(&session).await;
    let detail = execute_err(&session).await;
    assert!(detail.contains("build-failed"), "{detail}");
    assert!(detail.contains("mock build failure"), "typed detail preserved: {detail}");
}

#[tokio::test]
async fn merge_failure_parks_built() {
    let session = Session::scripted("mock-fail-merge", vec![mock::answers::greeting_synthesis()]);
    support::write_greeting_plan(session.root());
    support::refine_plan(&session).await;
    let detail = execute_err(&session).await;
    assert!(detail.contains("seam-dispatch-failed"), "{detail}");
    assert!(detail.contains("mock merge failure"), "typed detail preserved: {detail}");

    let metadata =
        std::fs::read_to_string(session.root().join(".emery/change/slices/greeting/metadata.yaml"))
            .expect("slice still present");
    assert!(metadata.contains("completed-at:"), "no commit happened:\n{metadata}");
    assert!(
        project::build_record::BuildRecord::present(
            &session.root().join(".emery/change/slices/greeting")
        ),
        "build record must remain when merge parks"
    );
    assert!(!session.root().join(".emery/specs/greeting/spec.md").exists(), "no baseline write");
}

#[tokio::test]
async fn missing_output_aborts() {
    let session =
        Session::scripted("mock-missing-output", vec![mock::answers::greeting_synthesis()]);
    support::write_greeting_plan(session.root());
    support::refine_plan(&session).await;
    let detail = execute_err(&session).await;
    assert!(
        detail.contains("target-build-output-missing") || detail.contains("output"),
        "{detail}"
    );
}
