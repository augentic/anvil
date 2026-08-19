//! Operations-trait dispatch and references-server identity.

use emery_adapter::answers::{EVIDENCE_ANSWER_SCHEMA, evidence_tail};
use emery_adapter::registry::Doc;
use emery_adapter::seam::{Context, Error, Evidence, SourceInput, SourceMetadata};
use emery_adapter::{AdapterIdentity, Model, Source, references, repaired};
use omnia_testkit::model::Harness;

const DOCS: &[Doc] = &[Doc {
    path: "prompts/extract.md",
    body: "EXTRACT",
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

    async fn extract<P: Model>(
        model: &P, ctx: &Context<'_>, input: &SourceInput,
    ) -> Result<Evidence, Error> {
        repaired(
            model,
            ctx,
            "SYSTEM".to_string(),
            input.key.clone(),
            "evidence",
            EVIDENCE_ANSWER_SCHEMA,
            evidence_tail,
        )
        .await
    }
}

fn ctx() -> Context<'static> {
    Context {
        adapter_id: "source:probe",
        project_root: std::path::Path::new("."),
        mcp_url: None,
        lend: Some(".".to_string()),
    }
}

#[tokio::test]
async fn source_dispatch() {
    let model = Harness::answering([
        r#"{"authority":"documentation","claims":[{"kind":"requirement","id":"one.claim"}]}"#,
    ]);

    let evidence = Probe::extract(&model, &ctx(), &SourceInput::value("main", ""))
        .await
        .expect("scripted extract succeeds");
    assert_eq!(evidence.claims.len(), 1);
    assert_eq!(evidence.claims[0].id.as_deref(), Some("one.claim"));

    assert_eq!(<Probe as Source>::IDENTITY.name, "probe");
    assert_eq!(<Probe as Source>::IDENTITY.version, "0.0.0");
    assert_eq!(<Probe as Source>::metadata(), SourceMetadata { emery_floor: None });
    assert_eq!(<Probe as Source>::docs()[0].path, "prompts/extract.md");
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
