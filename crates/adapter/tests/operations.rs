//! Operations-trait dispatch and references-server identity.

use adapter::answers::{EVIDENCE_ANSWER_SCHEMA, LEADS_ANSWER_SCHEMA, evidence_tail, leads_tail};
use adapter::registry::Doc;
use adapter::seam::{
    BuildContext, Context, Error, Evidence, Input, Lead, MergePhase, Report, SourceMetadata,
    TargetMetadata, Workspace,
};
use adapter::{AdapterIdentity, Model, Source, Target, references, repaired};
use omnia_testkit::model::Harness;

const DOCS: &[Doc] = &[Doc {
    path: "prompts/survey.md",
    body: "SURVEY",
}];

struct Probe;

impl Source for Probe {
    const IDENTITY: AdapterIdentity = AdapterIdentity {
        name: "probe",
        version: "0.0.0",
    };

    fn metadata() -> SourceMetadata {
        SourceMetadata { emery_floor: None }
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
    const IDENTITY: AdapterIdentity = AdapterIdentity {
        name: "probe",
        version: "0.0.0",
    };

    fn metadata() -> TargetMetadata {
        TargetMetadata {
            emery_floor: None,
            inputs: Vec::new(),
            platforms: None,
        }
    }

    fn docs() -> &'static [Doc] {
        DOCS
    }

    async fn guidance<P: Model>(_model: &P, ctx: &Context<'_>) -> Result<String, Error> {
        if ctx.adapter_id.contains("fail-guidance") {
            return Err(Error::Internal(format!("guidance failure for `{}`", ctx.adapter_id)));
        }
        Ok("GUIDANCE".to_string())
    }

    async fn build<P: Model>(
        _model: &P, _ctx: &Context<'_>, _slice: &str, _inputs: &[Input], _context: &BuildContext,
        _workspace: &Workspace,
    ) -> Result<Report, Error> {
        Ok(Report::success())
    }

    async fn merge<P: Model>(
        _model: &P, _ctx: &Context<'_>, _slice: &str, _phase: MergePhase, _workspace: &Workspace,
    ) -> Result<Report, Error> {
        Ok(Report::success())
    }
}

fn ctx() -> Context<'static> {
    Context {
        adapter_id: "source:probe",
        project_root: std::path::Path::new("."),
        mcp_url: None,
        lend: ".".to_string(),
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

    assert_eq!(<Probe as Source>::IDENTITY.name, "probe");
    assert_eq!(<Probe as Source>::IDENTITY.version, "0.0.0");
    assert_eq!(<Probe as Source>::metadata(), SourceMetadata { emery_floor: None });
    assert_eq!(<Probe as Source>::docs()[0].path, "prompts/survey.md");
}

#[tokio::test]
async fn target_dispatch() {
    let model = Harness::answering([""; 0]);
    let workspace = Workspace {
        id: "ws-1".to_string(),
        root: "/emery-workspaces/ws-1".to_string(),
        artifacts: "/host/project".to_string(),
    };

    let report = Probe::build(&model, &ctx(), "demo", &[], &BuildContext::default(), &workspace)
        .await
        .expect("build succeeds");
    assert_eq!(report, Report::success());
    let report = Probe::merge(&model, &ctx(), "demo", MergePhase::Preflight, &workspace)
        .await
        .expect("merge succeeds");
    assert_eq!(report, Report::success());
    let guidance = Probe::guidance(&model, &ctx()).await.expect("guidance succeeds");
    assert_eq!(guidance, "GUIDANCE");
}

// The WIT marks guidance fallible; a typed adapter error crosses the
// trait surface intact.
#[tokio::test]
async fn guidance_failure() {
    let model = Harness::answering([""; 0]);
    let failing = Context {
        adapter_id: "target:probe-fail-guidance",
        project_root: std::path::Path::new("."),
        mcp_url: None,
        lend: ".".to_string(),
    };
    let err = Probe::guidance(&model, &failing).await.expect_err("guidance fails");
    assert!(matches!(err, Error::Internal(detail) if detail.contains("probe-fail-guidance")));
}

#[test]
fn fn_pointer_coercion() {
    let metadata: fn() -> SourceMetadata = <Probe as Source>::metadata;
    let docs: fn() -> &'static [Doc] = <Probe as Source>::docs;
    assert_eq!(metadata(), SourceMetadata { emery_floor: None });
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
