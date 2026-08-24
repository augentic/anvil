//! Source adapter operation tests.

use emery_adapter::answers::{evidence_schema, evidence_tail};
use emery_adapter::registry::Doc;
use emery_adapter::types::{Context, Error, Evidence, SourceInput, SourceMetadata};
use emery_adapter::{Model, SourceAdapter, repaired};
use emery_testkit::Scripted;

const DOCS: &[Doc] = &[Doc {
    path: "prompts/extract.md",
    body: "EXTRACT",
}];

struct Probe;

impl SourceAdapter for Probe {
    const IDENTITY: &str = "probe@0.0.0";

    fn metadata() -> SourceMetadata {
        SourceMetadata { emery_floor: None }
    }

    fn docs() -> &'static [Doc] {
        DOCS
    }

    async fn extract<P: Model>(
        model: &P, ctx: &Context<'_>, input: &SourceInput,
    ) -> Result<Evidence, Error> {
        let schema = evidence_schema();
        repaired(
            model,
            ctx,
            "SYSTEM".to_string(),
            input.key.clone(),
            "evidence",
            &schema,
            evidence_tail,
        )
        .await
    }
}

#[tokio::test]
async fn source_dispatch() {
    let model = Scripted::answering([
        r#"{"authority":"documentation","claims":[{"kind":"requirement","id":"one.claim"}]}"#,
    ]);
    let ctx = Context {
        adapter_id: "source:probe",
        project_root: std::path::Path::new("."),
        docs: DOCS,
        lend: Some(".".to_string()),
    };

    let evidence = Probe::extract(&model, &ctx, &SourceInput::value("main", ""))
        .await
        .expect("scripted extract succeeds");
    assert_eq!(evidence.claims.len(), 1);
    assert_eq!(evidence.claims[0].id.as_deref(), Some("one.claim"));

    assert_eq!(<Probe as SourceAdapter>::IDENTITY, "probe@0.0.0");
    assert_eq!(<Probe as SourceAdapter>::metadata(), SourceMetadata { emery_floor: None });
    assert_eq!(<Probe as SourceAdapter>::docs()[0].path, "prompts/extract.md");
}

#[test]
fn fn_pointer_coercion() {
    let metadata: fn() -> SourceMetadata = <Probe as SourceAdapter>::metadata;
    let docs: fn() -> &'static [Doc] = <Probe as SourceAdapter>::docs;
    assert_eq!(metadata(), SourceMetadata { emery_floor: None });
    assert_eq!(docs().len(), 1);
}
