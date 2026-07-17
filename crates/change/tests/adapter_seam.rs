//! Typed adapter failures crossing the seam: each fixture failure
//! profile surfaces at the public operation boundary — an author
//! abort, a parked refine or build in the execute loop, or the
//! outputs-exist gate — with the adapter's typed detail preserved.

mod support;

use change::plan;
use fixture::session::Session;
use harness::invoke::run;

async fn author(
    session: &Session, source_adapter: &str,
) -> Result<plan::handlers::AuthorBody, project::handler::Error> {
    run::<plan::handlers::Author, _, _>(
        session.provider(),
        plan::handlers::AuthorInput {
            name: "demo".to_string(),
            sources: support::greeting_binding_for(source_adapter),
            intent: None,
        },
    )
    .await
}

async fn approve(session: &Session) {
    run::<plan::handlers::Transition, _, _>(
        session.provider(),
        plan::handlers::TransitionInput {
            name: "demo".to_string(),
            target: Some("approved".to_string()),
            undo: false,
            actor: "operator".to_string(),
        },
    )
    .await
    .expect("the operator stamps Gate 1");
}

async fn execute_err(session: &Session) -> String {
    run::<plan::handlers::Execute, _, _>(session.provider(), plan::handlers::ExecuteInput {})
        .await
        .expect_err("the failing phase parks the loop")
        .to_string()
}

#[tokio::test]
async fn survey_failure_aborts_author() {
    let session = Session::scripted("fixture", Vec::new());

    let err = author(&session, "fixture-fail-survey").await.expect_err("survey fails");
    let detail = err.to_string();
    assert!(detail.contains("survey"), "{detail}");
    assert!(detail.contains("fixture survey failure"), "typed detail preserved: {detail}");
    // The failure aborted before any judgment dispatch: the script is
    // empty, so a dispatch would have surfaced `model script exhausted`
    // instead of the survey failure above.
}

#[tokio::test]
async fn extract_failure_parks_refine() {
    let session = Session::scripted("fixture", vec![fixture::answers::greeting_grouping()]);

    author(&session, "fixture-fail-extract").await.expect("survey succeeds for this profile");
    approve(&session).await;

    let detail = execute_err(&session).await;
    assert!(detail.contains("refine-failed"), "{detail}");
    assert!(detail.contains("fixture extract failure"), "typed detail preserved: {detail}");
}

#[tokio::test]
async fn guidance_failure_parks_refine() {
    let session =
        Session::scripted("fixture-fail-guidance", vec![fixture::answers::greeting_grouping()]);

    author(&session, "fixture").await.expect("author succeeds");
    approve(&session).await;

    let detail = execute_err(&session).await;
    assert!(detail.contains("refine-failed"), "{detail}");
    assert!(detail.contains("fixture guidance failure"), "typed detail preserved: {detail}");
}

#[tokio::test]
async fn build_failure_parks() {
    let session = Session::scripted(
        "fixture-fail-build",
        vec![fixture::answers::greeting_grouping(), fixture::answers::greeting_synthesis()],
    );

    author(&session, "fixture").await.expect("author succeeds");
    approve(&session).await;

    let detail = execute_err(&session).await;
    assert!(detail.contains("build-failed"), "{detail}");
    assert!(detail.contains("fixture build failure"), "typed detail preserved: {detail}");
}

#[tokio::test]
async fn merge_failure_parks_built() {
    let session = Session::scripted(
        "fixture-fail-merge",
        vec![fixture::answers::greeting_grouping(), fixture::answers::greeting_synthesis()],
    );

    author(&session, "fixture").await.expect("author succeeds");
    approve(&session).await;

    // Refine and build succeed; the merge preflight dispatch itself
    // errors (a typed seam failure, not a failed gate report), so the
    // loop parks with the deterministic commit never attempted.
    let detail = execute_err(&session).await;
    assert!(detail.contains("seam-dispatch-failed"), "{detail}");
    assert!(detail.contains("fixture merge failure"), "typed detail preserved: {detail}");

    let metadata =
        std::fs::read_to_string(session.root().join(".specify/slices/greeting/metadata.yaml"))
            .expect("slice still present");
    assert!(metadata.contains("status: built"), "no commit happened:\n{metadata}");
    assert!(!session.root().join(".specify/specs/greeting/spec.md").exists(), "no baseline write");
}

#[tokio::test]
async fn missing_output_aborts() {
    let session = Session::scripted(
        "fixture-missing-output",
        vec![fixture::answers::greeting_grouping(), fixture::answers::greeting_synthesis()],
    );

    author(&session, "fixture").await.expect("author succeeds");
    approve(&session).await;

    // The fixture reports success but never writes its declared
    // output, so the orchestrator's outputs-exist gate aborts.
    let detail = execute_err(&session).await;
    assert!(detail.contains("target-build-output-missing"), "{detail}");
}
