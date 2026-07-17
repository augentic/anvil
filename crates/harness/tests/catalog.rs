//! Catalog builder and vtable dispatch over the shared probe
//! implementors.
//!
//! One native type may implement both axes (the one-axis rule binds
//! component exports, not native impls); the catalog still routes each
//! axis-qualified id to its own operation set.

mod support;

use adapter::seam::{Context, Error, Input, MergePhase, WorkingTree};
use harness::catalog::Catalog;
use omnia_testkit::model::Scripted;
use project::adapter::Axis;
use support::{FailGuidance, Floored, Probe};

fn linked() -> Catalog<Scripted> {
    Catalog::builder().source::<Probe>().target::<Probe>().build()
}

const fn ctx<'a>(id: &'a str, root: &'a std::path::Path) -> Context<'a> {
    Context {
        adapter_id: id,
        project_root: root,
        mcp_url: None,
    }
}

#[tokio::test]
async fn survey_threads_the_model() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let model = Scripted::answers([r#"{"answer":"password-reset"}"#]);
    let ctx = ctx("source:fixture", tmp.path());

    let leads = linked().survey(&model, &ctx, "source:fixture").await.expect("survey dispatches");

    assert_eq!(leads.len(), 1);
    assert_eq!(leads[0].lead, r#"{"answer":"password-reset"}"#);
    assert_eq!(leads[0].synopsis, "surveyed by source:fixture");
}

#[tokio::test]
async fn target_legs_dispatch() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let model = Scripted::answers::<&str>([]);
    let ctx = ctx("target:fixture", tmp.path());
    let tree = WorkingTree {
        base: "live".to_string(),
        subpath: None,
    };
    let linked = linked();

    let guidance =
        linked.guidance(&model, &ctx, "target:fixture").await.expect("guidance dispatches");
    assert_eq!(guidance, "fixture guidance");

    // The probe echoes its arguments through the report's single
    // output path, so the asserts prove the legs thread them intact.
    let inputs = vec![Input::Proposal("BODY".to_string())];
    let report = linked
        .build(&model, &ctx, "target:fixture", "demo", &inputs, &tree)
        .await
        .expect("build dispatches");
    assert_eq!(report.outputs[0].path, "build:demo:1");

    let report = linked
        .merge(&model, &ctx, "target:fixture", "demo", MergePhase::Preflight, &tree)
        .await
        .expect("merge dispatches");
    assert_eq!(report.outputs[0].path, "merge:demo:preflight");
}

// A typed guidance error crosses catalog dispatch intact. The routed
// id reaches the implementor through `ctx.adapter_id`, so the failing
// identity selects its own failure.
#[tokio::test]
async fn guidance_failure_propagates() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let model = Scripted::answers::<&str>([]);
    let ctx = ctx("target:fixture-fail-guidance", tmp.path());
    let linked: Catalog<Scripted> = Catalog::builder().target::<FailGuidance>().build();

    let err = linked
        .guidance(&model, &ctx, "target:fixture-fail-guidance")
        .await
        .expect_err("the failing identity fails guidance");
    assert!(
        matches!(err, Error::Internal(detail) if detail.contains("fixture-fail-guidance")),
        "the adapter's typed error survives catalog dispatch"
    );
}

#[tokio::test]
async fn axis_routing() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let model = Scripted::answers::<&str>([]);
    let ctx = ctx("target:fixture", tmp.path());
    let linked = linked();

    // A target id never reaches the source legs, and vice versa.
    let err = linked.survey(&model, &ctx, "target:fixture").await.expect_err("axis mismatch");
    assert!(matches!(err, Error::InvalidRequest(detail) if detail.contains("target:fixture")));
    let err = linked.guidance(&model, &ctx, "source:fixture").await.expect_err("axis mismatch");
    assert!(matches!(err, Error::InvalidRequest(_)));

    // Unlinked ids refuse on both axes.
    let err = linked.survey(&model, &ctx, "source:unknown").await.expect_err("unlinked");
    assert!(matches!(err, Error::InvalidRequest(detail) if detail.contains("source:unknown")));
}

#[test]
fn entries_and_metadata() {
    let linked: Catalog<Scripted> =
        Catalog::builder().source::<Probe>().target::<Probe>().target::<Floored>().build();
    let ids: Vec<String> = linked.entries().iter().map(harness::catalog::Entry::id).collect();
    assert_eq!(ids, ["source:fixture", "target:fixture", "target:floored"]);

    let entry = linked.get(Axis::Target, "fixture").expect("target entry");
    assert_eq!(entry.server_name(), "fixture-references");
    assert_eq!(entry.metadata().specify_floor, None);
    assert!(!entry.docs().is_empty());

    let floored = linked.get(Axis::Target, "floored").expect("floored entry");
    assert_eq!(floored.metadata().specify_floor.as_deref(), Some("9.9.9"));

    let err = linked.get(Axis::Source, "unknown").expect_err("unlinked refuses");
    assert_eq!(err.variant_str(), "adapter-not-found");
}
