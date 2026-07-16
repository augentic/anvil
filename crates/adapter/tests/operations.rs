//! Operations-trait dispatch and references-server identity.

use adapter::answers::{EVIDENCE_ANSWER_SCHEMA, LEADS_ANSWER_SCHEMA, evidence_tail, leads_tail};
use adapter::registry::Doc;
use adapter::seam::{
    Context, Error, Evidence, Input, Lead, MergePhase, Report, SourceMetadata, TargetMetadata,
    WorkingTree,
};
use adapter::{Model, Source, Target, references, repaired};
use testkit::Harness;

const DOCS: &[Doc] = &[Doc {
    path: "prompts/survey.md",
    body: "SURVEY",
}];

struct Probe;

impl Source for Probe {
    const NAME: &'static str = "probe";

    fn metadata() -> SourceMetadata {
        SourceMetadata { specify_floor: None }
    }

    fn docs() -> &'static [Doc] {
        DOCS
    }

    async fn survey<P: Model>(model: &P, ctx: &Context<'_>) -> Result<Vec<Lead>, Error> {
        repaired(
            model,
            ctx,
            "SYSTEM".to_string(),
            "USER".to_string(),
            "leads",
            LEADS_ANSWER_SCHEMA,
            leads_tail,
        )
        .await
    }

    async fn extract<P: Model>(
        model: &P, ctx: &Context<'_>, lead: &Lead,
    ) -> Result<Evidence, Error> {
        repaired(
            model,
            ctx,
            "SYSTEM".to_string(),
            lead.render(),
            "evidence",
            EVIDENCE_ANSWER_SCHEMA,
            evidence_tail,
        )
        .await
    }
}

impl Target for Probe {
    const NAME: &'static str = "probe";

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

    fn guidance() -> &'static str {
        "GUIDANCE"
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

fn ctx() -> Context<'static> {
    Context {
        adapter_id: "source:probe",
        project_root: std::path::Path::new("."),
        mcp_url: None,
    }
}

async fn survey_of<A: Source, M: Model>(model: &M, ctx: &Context<'_>) -> Result<Vec<Lead>, Error> {
    A::survey(model, ctx).await
}

#[tokio::test]
async fn source_dispatch() {
    let model = Harness::answering([r#"{"leads":[{"lead":"one","synopsis":"the lead"}]}"#]);

    let leads = survey_of::<Probe, _>(&model, &ctx()).await.expect("scripted survey succeeds");
    assert_eq!(leads.len(), 1);
    assert_eq!(leads[0].lead, "one");

    assert_eq!(<Probe as Source>::NAME, "probe");
    assert_eq!(<Probe as Source>::metadata(), SourceMetadata { specify_floor: None });
    assert_eq!(<Probe as Source>::docs()[0].path, "prompts/survey.md");
}

#[tokio::test]
async fn target_dispatch() {
    let model = Harness::answering([""; 0]);
    let tree = WorkingTree {
        base: "rev-1".to_string(),
        subpath: None,
    };

    let report = Probe::build(&model, &ctx(), "demo", &[], &tree).await.expect("build succeeds");
    assert_eq!(report, Report::success());
    let report = Probe::merge(&model, &ctx(), "demo", MergePhase::Preflight, &tree)
        .await
        .expect("merge succeeds");
    assert_eq!(report, Report::success());
    assert_eq!(Probe::guidance(), "GUIDANCE");
}

#[test]
fn fn_pointer_coercion() {
    let metadata: fn() -> SourceMetadata = <Probe as Source>::metadata;
    let docs: fn() -> &'static [Doc] = <Probe as Source>::docs;
    assert_eq!(metadata(), SourceMetadata { specify_floor: None });
    assert_eq!(docs().len(), 1);
}

#[test]
fn server_name() {
    let first = references::server_name("captures");
    assert_eq!(first, "captures-references");
    // Interned: the projection returns the same allocation every call.
    assert!(std::ptr::eq(first, references::server_name("captures")));
    assert_eq!(references::server_name("some-adapter"), "some-adapter-references");
}
