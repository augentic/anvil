//! Typed adapter failures crossing the seam: each fixture failure
//! profile surfaces at the public operation boundary — an author
//! abort, a parked refine or build in the execute loop, or the
//! outputs-exist gate — with the adapter's typed detail preserved.

use change::plan;
use omnia_guest::api::invoke::Invoker;

mod common;

use common::answers;
use common::fixture::{ScriptedProvider, run, scripted_invoker, scripted_project};

async fn author(
    invoker: &Invoker<ScriptedProvider>, source_adapter: &str,
) -> Result<plan::handlers::AuthorBody, project::handler::Error> {
    run::<plan::handlers::Author, _>(
        invoker,
        plan::handlers::AuthorInput {
            name: "demo".to_string(),
            sources: answers::greeting_binding_for(source_adapter),
            intent: None,
        },
    )
    .await
}

async fn approve(invoker: &Invoker<ScriptedProvider>) {
    run::<plan::handlers::Transition, _>(
        invoker,
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

async fn execute_err(invoker: &Invoker<ScriptedProvider>) -> String {
    run::<plan::handlers::Execute, _>(invoker, plan::handlers::ExecuteInput {})
        .await
        .expect_err("the failing phase parks the loop")
        .to_string()
}

#[tokio::test]
async fn survey_failure_aborts_author() {
    let (_tmp, root, _cache) = scripted_project("fixture");
    let invoker = scripted_invoker(&root, Vec::new());

    let err = author(&invoker, "fixture-fail-survey").await.expect_err("survey fails");
    let detail = err.to_string();
    assert!(detail.contains("survey"), "{detail}");
    assert!(detail.contains("fixture survey failure"), "typed detail preserved: {detail}");
    // The failure aborted before any judgment dispatch.
    invoker.provider().model().assert_exhausted();
}

#[tokio::test]
async fn extract_failure_parks_refine() {
    let (_tmp, root, _cache) = scripted_project("fixture");
    let invoker = scripted_invoker(&root, vec![answers::greeting_grouping()]);

    author(&invoker, "fixture-fail-extract").await.expect("survey succeeds for this profile");
    approve(&invoker).await;

    let detail = execute_err(&invoker).await;
    assert!(detail.contains("refine-failed"), "{detail}");
    assert!(detail.contains("fixture extract failure"), "typed detail preserved: {detail}");
    invoker.provider().model().assert_exhausted();
}

#[tokio::test]
async fn guidance_failure_parks_refine() {
    let (_tmp, root, _cache) = scripted_project("fixture-fail-guidance");
    let invoker = scripted_invoker(&root, vec![answers::greeting_grouping()]);

    author(&invoker, "fixture").await.expect("author succeeds");
    approve(&invoker).await;

    let detail = execute_err(&invoker).await;
    assert!(detail.contains("refine-failed"), "{detail}");
    assert!(detail.contains("fixture guidance failure"), "typed detail preserved: {detail}");
    invoker.provider().model().assert_exhausted();
}

#[tokio::test]
async fn build_failure_parks() {
    let (_tmp, root, _cache) = scripted_project("fixture-fail-build");
    let invoker =
        scripted_invoker(&root, vec![answers::greeting_grouping(), answers::greeting_synthesis()]);

    author(&invoker, "fixture").await.expect("author succeeds");
    approve(&invoker).await;

    let detail = execute_err(&invoker).await;
    assert!(detail.contains("build-failed"), "{detail}");
    assert!(detail.contains("fixture build failure"), "typed detail preserved: {detail}");
    invoker.provider().model().assert_exhausted();
}

#[tokio::test]
async fn merge_failure_parks_built() {
    let (_tmp, root, _cache) = scripted_project("fixture-fail-merge");
    let invoker =
        scripted_invoker(&root, vec![answers::greeting_grouping(), answers::greeting_synthesis()]);

    author(&invoker, "fixture").await.expect("author succeeds");
    approve(&invoker).await;

    // Refine and build succeed; the merge preflight dispatch itself
    // errors (a typed seam failure, not a failed gate report), so the
    // loop parks with the deterministic commit never attempted.
    let detail = execute_err(&invoker).await;
    assert!(detail.contains("seam-dispatch-failed"), "{detail}");
    assert!(detail.contains("fixture merge failure"), "typed detail preserved: {detail}");

    let metadata = std::fs::read_to_string(root.join(".specify/slices/greeting/metadata.yaml"))
        .expect("slice still present");
    assert!(metadata.contains("status: built"), "no commit happened:\n{metadata}");
    assert!(!root.join(".specify/specs/greeting/spec.md").exists(), "no baseline write");
    invoker.provider().model().assert_exhausted();
}

#[tokio::test]
async fn missing_output_aborts() {
    let (_tmp, root, _cache) = scripted_project("fixture-missing-output");
    let invoker =
        scripted_invoker(&root, vec![answers::greeting_grouping(), answers::greeting_synthesis()]);

    author(&invoker, "fixture").await.expect("author succeeds");
    approve(&invoker).await;

    // The fixture reports success but never writes its declared
    // output, so the orchestrator's outputs-exist gate aborts.
    let detail = execute_err(&invoker).await;
    assert!(detail.contains("target-build-output-missing"), "{detail}");
    invoker.provider().model().assert_exhausted();
}
