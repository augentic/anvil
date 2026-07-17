//! The exhaustive catalog inventory and the failure profiles, all
//! exercised through the SDK trait surface (no provider hooks).

use adapter::seam::{Context, Error, Input, MergePhase, Status, WorkingTree};
use adapter::{Source, Target};
use fixture::{
    Adapter, FailBuild, FailExtract, FailGuidance, FailMerge, FailSurvey, MissingOutput, catalog,
};
use omnia_testkit::model::Scripted;
use project::adapter::Axis;

// The full fixture registry, by `(axis, name)` — a new identity or a
// renamed one must update this inventory deliberately.
#[test]
fn inventory() {
    let catalog = catalog::<Scripted>();
    let entries: Vec<(Axis, &str)> =
        catalog.entries().iter().map(|entry| (entry.axis(), entry.name())).collect();
    assert_eq!(
        entries,
        vec![
            (Axis::Source, "fixture"),
            (Axis::Source, "fixture-docs"),
            (Axis::Source, "fixture-code"),
            (Axis::Source, "fixture-fail-survey"),
            (Axis::Source, "fixture-fail-extract"),
            (Axis::Target, "fixture"),
            (Axis::Target, "fixture-fail-guidance"),
            (Axis::Target, "fixture-fail-build"),
            (Axis::Target, "fixture-fail-merge"),
            (Axis::Target, "fixture-missing-output"),
        ]
    );
}

fn ctx(id: &str) -> Context<'_> {
    Context::guest(id, None)
}

fn model() -> Scripted {
    Scripted::answers::<&str>([])
}

fn tree() -> WorkingTree {
    WorkingTree {
        base: "HEAD".to_string(),
        subpath: None,
    }
}

#[tokio::test]
async fn source_failures() {
    let model = model();

    let err = FailSurvey::survey(&model, &ctx("source:fixture-fail-survey"))
        .await
        .expect_err("fail-survey fails");
    assert!(matches!(err, Error::Internal(detail) if detail.contains("fixture-fail-survey")));

    // The failing identity still surveys nothing before extract: the
    // extract failure is its own typed error.
    let lead = fixture::behaviour::survey("source:fixture").expect("minimal survey").remove(0);
    let err = FailExtract::extract(&model, &ctx("source:fixture-fail-extract"), &lead)
        .await
        .expect_err("fail-extract fails");
    assert!(matches!(err, Error::Internal(detail) if detail.contains("fixture-fail-extract")));
}

#[tokio::test]
async fn target_failures() {
    let model = model();
    let tmp = tempfile::tempdir().expect("tempdir");
    let _guard = fixture::Cwd::enter(tmp.path());

    let err = FailGuidance::guidance(&model, &ctx("target:fixture-fail-guidance"))
        .await
        .expect_err("fail-guidance fails through the trait surface");
    assert!(matches!(err, Error::Internal(detail) if detail.contains("fixture-fail-guidance")));

    let err = FailBuild::build(&model, &ctx("target:fixture-fail-build"), "s", &[], &tree())
        .await
        .expect_err("fail-build fails");
    assert!(matches!(err, Error::Internal(detail) if detail.contains("fixture-fail-build")));

    let err = FailMerge::merge(
        &model,
        &ctx("target:fixture-fail-merge"),
        "s",
        MergePhase::Preflight,
        &tree(),
    )
    .await
    .expect_err("fail-merge fails");
    assert!(matches!(err, Error::Internal(detail) if detail.contains("fixture-fail-merge")));
}

// The dishonest success: a success report declaring an output that was
// never written, for the caller's outputs-exist gate.
#[tokio::test]
async fn missing_output_reports_unwritten_path() {
    let model = model();
    let tmp = tempfile::tempdir().expect("tempdir");
    let _guard = fixture::Cwd::enter(tmp.path());

    let report = MissingOutput::build(
        &model,
        &ctx("target:fixture-missing-output"),
        "greeting",
        &[],
        &tree(),
    )
    .await
    .expect("missing-output reports success");
    assert_eq!(report.status, Status::Success);
    assert_eq!(report.outputs.len(), 1);
    assert!(!tmp.path().join(&report.outputs[0].path).exists(), "the output is never written");
}

// The happy path writes the observable artifact and reports it.
#[tokio::test]
async fn build_writes_artifact() {
    let model = model();
    let tmp = tempfile::tempdir().expect("tempdir");
    let _guard = fixture::Cwd::enter(tmp.path());

    let inputs = [Input::Proposal("# p".to_string()), Input::Spec("## s".to_string())];
    let report = Adapter::build(&model, &ctx("target:fixture"), "greeting", &inputs, &tree())
        .await
        .expect("fixture builds");
    assert_eq!(report.status, Status::Success);

    let artifact = fixture::behaviour::build_artifact_path(tmp.path(), "greeting");
    let body = std::fs::read_to_string(artifact).expect("artifact written");
    assert!(body.contains("proposal 1"));
    assert!(body.contains("specs 1"));
}
