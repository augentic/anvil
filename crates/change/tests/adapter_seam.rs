//! Typed adapter failures crossing the seam: each fixture failure
//! profile surfaces at the public operation boundary — an author
//! abort, a parked refine or build in the execute loop, or the
//! outputs-exist gate — with the adapter's typed detail preserved.

use std::path::{Path, PathBuf};

use change::plan;
use testkit::{ReplayProvider, answers, run};

/// The committed replay fixtures for one test — regenerate with
/// `REGENERATE_FIXTURES=1 cargo nextest run -p change adapter_seam`.
fn fixtures(test: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/replay/adapter_seam").join(test)
}

async fn author(
    provider: &ReplayProvider, source_adapter: &str,
) -> Result<plan::handlers::AuthorBody, project::handler::Error> {
    run::<plan::handlers::Author, _, _>(
        provider,
        plan::handlers::AuthorInput {
            name: "demo".to_string(),
            sources: answers::greeting_binding_for(source_adapter),
            intent: None,
        },
    )
    .await
}

async fn approve(provider: &ReplayProvider) {
    run::<plan::handlers::Transition, _, _>(
        provider,
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

async fn execute_err(provider: &ReplayProvider) -> String {
    run::<plan::handlers::Execute, _, _>(provider, plan::handlers::ExecuteInput {})
        .await
        .expect_err("the failing phase parks the loop")
        .to_string()
}

#[tokio::test]
async fn survey_failure_aborts_author() {
    let provider = ReplayProvider::replay("fixture", &fixtures("survey_fails"), Vec::new());

    let err = author(&provider, "fixture-fail-survey").await.expect_err("survey fails");
    let detail = err.to_string();
    assert!(detail.contains("survey"), "{detail}");
    assert!(detail.contains("fixture survey failure"), "typed detail preserved: {detail}");
    // The failure aborted before any judgment dispatch.
    assert!(provider.model().requests().is_empty());
}

#[tokio::test]
async fn extract_failure_parks_refine() {
    let provider = ReplayProvider::replay(
        "fixture",
        &fixtures("extract_fails"),
        vec![answers::greeting_grouping()],
    );

    author(&provider, "fixture-fail-extract").await.expect("survey succeeds for this profile");
    approve(&provider).await;

    let detail = execute_err(&provider).await;
    assert!(detail.contains("refine-failed"), "{detail}");
    assert!(detail.contains("fixture extract failure"), "typed detail preserved: {detail}");
}

#[tokio::test]
async fn guidance_failure_parks_refine() {
    let provider = ReplayProvider::replay(
        "fixture-fail-guidance",
        &fixtures("guidance_fails"),
        vec![answers::greeting_grouping()],
    );

    author(&provider, "fixture").await.expect("author succeeds");
    approve(&provider).await;

    let detail = execute_err(&provider).await;
    assert!(detail.contains("refine-failed"), "{detail}");
    assert!(detail.contains("fixture guidance failure"), "typed detail preserved: {detail}");
}

#[tokio::test]
async fn build_failure_parks() {
    let provider = ReplayProvider::replay(
        "fixture-fail-build",
        &fixtures("build_fails"),
        vec![answers::greeting_grouping(), answers::greeting_synthesis()],
    );

    author(&provider, "fixture").await.expect("author succeeds");
    approve(&provider).await;

    let detail = execute_err(&provider).await;
    assert!(detail.contains("build-failed"), "{detail}");
    assert!(detail.contains("fixture build failure"), "typed detail preserved: {detail}");
}

#[tokio::test]
async fn merge_failure_parks_built() {
    let provider = ReplayProvider::replay(
        "fixture-fail-merge",
        &fixtures("merge_fails"),
        vec![answers::greeting_grouping(), answers::greeting_synthesis()],
    );

    author(&provider, "fixture").await.expect("author succeeds");
    approve(&provider).await;

    // Refine and build succeed; the merge preflight dispatch itself
    // errors (a typed seam failure, not a failed gate report), so the
    // loop parks with the deterministic commit never attempted.
    let detail = execute_err(&provider).await;
    assert!(detail.contains("seam-dispatch-failed"), "{detail}");
    assert!(detail.contains("fixture merge failure"), "typed detail preserved: {detail}");

    let metadata =
        std::fs::read_to_string(provider.root.join(".specify/slices/greeting/metadata.yaml"))
            .expect("slice still present");
    assert!(metadata.contains("status: built"), "no commit happened:\n{metadata}");
    assert!(!provider.root.join(".specify/specs/greeting/spec.md").exists(), "no baseline write");
}

#[tokio::test]
async fn missing_output_aborts() {
    let provider = ReplayProvider::replay(
        "fixture-missing-output",
        &fixtures("missing_output"),
        vec![answers::greeting_grouping(), answers::greeting_synthesis()],
    );

    author(&provider, "fixture").await.expect("author succeeds");
    approve(&provider).await;

    // The fixture reports success but never writes its declared
    // output, so the orchestrator's outputs-exist gate aborts.
    let detail = execute_err(&provider).await;
    assert!(detail.contains("target-build-output-missing"), "{detail}");
}
