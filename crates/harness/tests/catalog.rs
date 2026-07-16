//! Catalog builder and vtable dispatch over a minimal in-test
//! implementor of both operations traits.
//!
//! One native type may implement both axes (the one-axis rule binds
//! component exports, not native impls); the catalog still routes each
//! axis-qualified id to its own operation set.

use adapter::registry::Doc;
use adapter::seam::{
    Context, Error, Evidence, Input, Lead, MergePhase, Report, SourceMetadata, TargetMetadata,
    WorkingTree,
};
use adapter::{Source, Target};
use harness::catalog::Catalog;
use omnia_guest::Model;
use omnia_guest::model::{Format, Request};
use omnia_testkit::model::Scripted;
use project::adapter::Axis;
use project::adapter::metadata::Request as MetadataRequest;

struct Fixture;

const DOCS: &[Doc] = &[Doc {
    path: "prompts/guidance.md",
    body: "fixture guidance",
}];

impl Source for Fixture {
    const NAME: &'static str = "fixture";

    fn metadata() -> SourceMetadata {
        SourceMetadata { specify_floor: None }
    }

    fn docs() -> &'static [Doc] {
        DOCS
    }

    async fn survey<P: Model>(model: &P, ctx: &Context<'_>) -> Result<Vec<Lead>, Error> {
        let reply = model
            .create(Request {
                format: Format::Json,
                ..Request::default()
            })
            .await
            .map_err(Error::from)?;
        Ok(vec![Lead {
            lead: reply.answer,
            synopsis: format!("surveyed by {}", ctx.adapter_id),
            topics: Vec::new(),
        }])
    }

    async fn extract<P: Model>(
        _model: &P, _ctx: &Context<'_>, lead: &Lead,
    ) -> Result<Evidence, Error> {
        Err(Error::Internal(format!("no evidence for {}", lead.lead)))
    }
}

impl Target for Fixture {
    const NAME: &'static str = "fixture";

    fn metadata() -> TargetMetadata {
        TargetMetadata {
            specify_floor: Some("9.9.9".to_string()),
            inputs: Vec::new(),
            platforms: None,
        }
    }

    fn docs() -> &'static [Doc] {
        DOCS
    }

    fn guidance() -> &'static str {
        "fixture guidance"
    }

    async fn build<P: Model>(
        _model: &P, _ctx: &Context<'_>, slice: &str, inputs: &[Input], _tree: &WorkingTree,
    ) -> Result<Report, Error> {
        assert_eq!(slice, "demo");
        assert_eq!(inputs.len(), 1);
        Ok(Report::success())
    }

    async fn merge<P: Model>(
        _model: &P, _ctx: &Context<'_>, _slice: &str, phase: MergePhase, _tree: &WorkingTree,
    ) -> Result<Report, Error> {
        assert_eq!(phase, MergePhase::Preflight);
        Ok(Report::success())
    }
}

fn linked() -> Catalog<Scripted> {
    Catalog::builder().source::<Fixture>().target::<Fixture>().build()
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

    assert_eq!(linked.guidance("target:fixture").expect("guidance"), "fixture guidance");

    let inputs = vec![Input::Proposal("BODY".to_string())];
    let report = linked
        .build(&model, &ctx, "target:fixture", "demo", &inputs, &tree)
        .await
        .expect("build dispatches");
    assert_eq!(report, Report::success());

    let report = linked
        .merge(&model, &ctx, "target:fixture", "demo", MergePhase::Preflight, &tree)
        .await
        .expect("merge dispatches");
    assert_eq!(report, Report::success());
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
    let err = linked.guidance("source:fixture").expect_err("axis mismatch");
    assert!(matches!(err, Error::InvalidRequest(_)));

    // Unlinked ids refuse on both axes.
    let err = linked.survey(&model, &ctx, "source:unknown").await.expect_err("unlinked");
    assert!(matches!(err, Error::InvalidRequest(detail) if detail.contains("source:unknown")));
}

#[test]
fn entries_and_metadata() {
    let linked = linked();
    let ids: Vec<String> = linked.entries().iter().map(harness::catalog::Entry::id).collect();
    assert_eq!(ids, ["source:fixture", "target:fixture"]);

    let entry = linked.get(Axis::Target, "fixture").expect("target entry");
    assert_eq!(entry.server_name(), "fixture-references");
    assert_eq!(entry.metadata().specify_floor.as_deref(), Some("9.9.9"));
    assert!(!entry.docs().is_empty());

    let metadata = linked
        .metadata(&MetadataRequest {
            axis: Axis::Source,
            adapter_id: "source:fixture",
        })
        .expect("source metadata");
    assert_eq!(metadata.specify_floor, None);

    let err = linked.get(Axis::Source, "unknown").expect_err("unlinked refuses");
    assert_eq!(err.variant_str(), "adapter-not-found");
}
