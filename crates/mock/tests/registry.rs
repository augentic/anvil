//! The exhaustive catalog inventory and the failure profiles, all
//! exercised through the SDK trait surface (no provider hooks).

use adapter::seam::{
    BuildContext, Context, Error, Input, MergePhase, Payload, PhaseOutcome, Workspace,
};
use adapter::{Source, Target};
use mock::{
    Adapter, FailBuild, FailExtract, FailGuidance, FailMerge, FailSurvey, MissingOutput, catalog,
};
use omnia_testkit::model::Scripted;
use project::adapter::Axis;

// The full mock registry, by `(axis, name)` — a new identity or a
// renamed one must update this inventory deliberately.
#[test]
fn inventory() {
    let catalog = catalog();
    let entries: Vec<(Axis, &str)> =
        catalog.entries().iter().map(|entry| (entry.axis(), entry.name())).collect();
    assert_eq!(
        entries,
        vec![
            (Axis::Source, "mock"),
            (Axis::Source, "mock-docs"),
            (Axis::Source, "mock-code"),
            (Axis::Source, "mock-fail-survey"),
            (Axis::Source, "mock-fail-extract"),
            (Axis::Target, "mock"),
            (Axis::Target, "mock-fail-guidance"),
            (Axis::Target, "mock-fail-build"),
            (Axis::Target, "mock-fail-merge"),
            (Axis::Target, "mock-missing-output"),
            (Axis::Target, "mock-tool-source"),
            (Axis::Target, "mock-verify-outputs"),
            (Axis::Target, "mock-na-blocking"),
            (Axis::Target, "mock-oversized-continuation"),
            (Axis::Target, "mock-stage-escape"),
            (Axis::Target, "mock-verify-continuation"),
        ]
    );
}

fn ctx(id: &str) -> Context<'_> {
    Context {
        adapter_id: id,
        project_root: std::path::Path::new("."),
        mcp_url: None,
        lend: ".".to_string(),
    }
}

fn model() -> Scripted {
    Scripted::answers::<&str>([])
}

/// A prepared workspace rooted at `root` — the mock build writes its
/// artifact into the workspace root, never the ambient checkout.
fn workspace(root: &std::path::Path) -> Workspace {
    Workspace {
        id: "ws-mock".to_string(),
        root: root.display().to_string(),
        artifacts: root.display().to_string(),
        artifact_stage: None,
    }
}

#[tokio::test]
async fn source_failures() {
    let model = model();

    let err = FailSurvey::survey(&model, &ctx("source:mock-fail-survey"))
        .await
        .expect_err("fail-survey fails");
    assert!(matches!(err, Error::Internal(detail) if detail.contains("mock-fail-survey")));

    // The failing identity still surveys nothing before extract: the
    // extract failure is its own typed error.
    let lead = mock::behaviour::survey("source:mock").expect("minimal survey").remove(0);
    let err = FailExtract::extract(&model, &ctx("source:mock-fail-extract"), &lead)
        .await
        .expect_err("fail-extract fails");
    assert!(matches!(err, Error::Internal(detail) if detail.contains("mock-fail-extract")));
}

#[tokio::test]
async fn target_failures() {
    let model = model();
    let tmp = tempfile::tempdir().expect("tempdir");
    let _guard = mock::Cwd::enter(tmp.path());

    let err = FailGuidance::guidance(&model, &ctx("target:mock-fail-guidance"))
        .await
        .expect_err("fail-guidance fails through the trait surface");
    assert!(matches!(err, Error::Internal(detail) if detail.contains("mock-fail-guidance")));

    let err = FailBuild::build(
        &model,
        &ctx("target:mock-fail-build"),
        "s",
        &[],
        &BuildContext::default(),
        &workspace(tmp.path()),
    )
    .await
    .expect_err("fail-build fails");
    assert!(matches!(err, Error::Internal(detail) if detail.contains("mock-fail-build")));

    let err = FailMerge::merge(
        &model,
        &ctx("target:mock-fail-merge"),
        "s",
        MergePhase::Preflight,
        &workspace(tmp.path()),
    )
    .await
    .expect_err("fail-merge fails");
    assert!(matches!(err, Error::Internal(detail) if detail.contains("mock-fail-merge")));
}

// The dishonest success: a success report declaring an output that was
// never written, for the caller's outputs-exist gate.
#[tokio::test]
async fn missing_output_names_path() {
    let model = model();
    let tmp = tempfile::tempdir().expect("tempdir");
    let _guard = mock::Cwd::enter(tmp.path());

    let report = MissingOutput::build(
        &model,
        &ctx("target:mock-missing-output"),
        "greeting",
        &[],
        &BuildContext::default(),
        &workspace(tmp.path()),
    )
    .await
    .expect("missing-output reports completion");
    assert_eq!(report.outcome, PhaseOutcome::Completed);
    assert_eq!(report.outputs.len(), 1);
    assert!(!tmp.path().join(&report.outputs[0].path).exists(), "the output is never written");
}

// The happy path writes the observable artifact and reports it.
#[tokio::test]
async fn build_writes_artifact() {
    let model = model();
    let tmp = tempfile::tempdir().expect("tempdir");
    let _guard = mock::Cwd::enter(tmp.path());

    let payload = |path: &str| Payload::Path(path.to_string());
    let inputs = [
        Input::Proposal(payload(".emery/slices/greeting/proposal.md")),
        Input::Spec(payload(".emery/slices/greeting/specs/core/spec.md")),
    ];
    let report = Adapter::build(
        &model,
        &ctx("target:mock"),
        "greeting",
        &inputs,
        &BuildContext::default(),
        &workspace(tmp.path()),
    )
    .await
    .expect("mock builds");
    assert_eq!(report.outcome, PhaseOutcome::Completed);

    let artifact = mock::behaviour::build_artifact_path(tmp.path(), "greeting");
    let body = std::fs::read_to_string(artifact).expect("artifact written");
    assert!(body.contains("proposal 1"));
    assert!(body.contains("specs 1"));
}
