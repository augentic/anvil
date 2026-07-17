//! Shelf mounting over the linked catalog: one implementor registered
//! on both axes (legal for native impls) shares one docs registry and
//! must mount its `/mcp/<name>` shelf once, not panic the merge.

use adapter::registry::Doc;
use adapter::seam::{
    Context, Error, Evidence, Input, Lead, MergePhase, Report, SourceMetadata, TargetMetadata,
    WorkingTree,
};
use adapter::{Source, Target};
use harness::catalog::Catalog;
use harness::mcp;
use omnia_guest::Model;
use omnia_testkit::model::Scripted;

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

    async fn survey<P: Model>(_model: &P, _ctx: &Context<'_>) -> Result<Vec<Lead>, Error> {
        Ok(Vec::new())
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
            specify_floor: None,
            inputs: Vec::new(),
            platforms: None,
        }
    }

    fn docs() -> &'static [Doc] {
        DOCS
    }

    async fn guidance<P: Model>(_model: &P, _ctx: &Context<'_>) -> Result<String, Error> {
        Ok("fixture guidance".to_string())
    }

    async fn build<P: Model>(
        _model: &P, _ctx: &Context<'_>, _slice: &str, _inputs: &[Input], _tree: &WorkingTree,
    ) -> Result<Report, Error> {
        Ok(Report::success())
    }

    async fn merge<P: Model>(
        _model: &P, _ctx: &Context<'_>, _slice: &str, _phase: MergePhase, _tree: &WorkingTree,
    ) -> Result<Report, Error> {
        Ok(Report::success())
    }
}

#[test]
fn dual_axis_shelf_mounts_once() {
    let linked: Catalog<Scripted> =
        Catalog::builder().source::<Fixture>().target::<Fixture>().build();

    let shelves = mcp::shelves(&linked);
    assert_eq!(shelves.len(), 2, "both axis entries expose the shared shelf");
    assert!(shelves.iter().all(|shelf| shelf.name == "fixture"));

    // Would panic on a duplicate `/mcp/fixture` mount before dedup.
    let _router = mcp::router(&linked);
}
