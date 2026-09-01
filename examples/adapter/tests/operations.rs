//! Greeting extract operation behavior over the `Source` capability.

use std::path::Path;

use emery_adapter::answers::evidence_schema;
use emery_adapter::registry::Doc;
use emery_adapter::types::{
    Authority, ClaimKind, Context, Error, SourceContent, SourceInput, SourceWorkspace,
};
use emery_adapter::{Format, Request, SourceAdapter as _};
use emery_testkit::{Scripted, function_tools};
use source::Adapter;

fn ctx(docs: &'static [Doc]) -> Context<'static> {
    Context {
        adapter_id: "source:source",
        project_root: Path::new("."),
        docs,
        lend: Some(".".to_string()),
    }
}

fn workspace_input() -> SourceInput {
    SourceInput {
        key: "greeting".to_string(),
        content: SourceContent::Workspace(SourceWorkspace {
            id: "view-1".to_string(),
            root: ".".to_string(),
        }),
    }
}

fn schema_format(request: &Request) -> (&str, &str) {
    match &request.format {
        Format::Schema(schema) => (&schema.name, &schema.schema),
        other => panic!("expected schema format, got {other:?}"),
    }
}

#[tokio::test]
async fn extract_leg() {
    let model = Scripted::answering([r#"{
            "authority": "documentation",
            "claims": [
                {"kind": "requirement", "id": "greeting.behaviour", "path": "greeting.md#L3", "statement": "GET /greeting returns the static string 'hello'."},
                {"kind": "criterion", "id": "greeting.behaviour.body", "path": "greeting.md#L6", "criterion": "The response body is exactly `hello`."}
            ]
        }"#]);

    let evidence =
        Adapter::extract(&model, &ctx(Adapter::docs()), &workspace_input()).await.unwrap();

    assert_eq!(evidence.authority, Authority::Documentation);
    assert_eq!(evidence.claims.len(), 2);
    assert_eq!(evidence.claims[0].kind, ClaimKind::Requirement);
    assert_eq!(evidence.claims[0].id.as_deref(), Some("greeting.behaviour"));
    assert_eq!(
        evidence.claims[0].extras.get("statement").and_then(|value| value.as_str()),
        Some("GET /greeting returns the static string 'hello'."),
    );
    assert_eq!(evidence.claims[1].kind, ClaimKind::Criterion);
    assert_eq!(evidence.claims[1].id.as_deref(), Some("greeting.behaviour.body"));
    assert_eq!(
        evidence.claims[1].extras.get("criterion").and_then(|value| value.as_str()),
        Some("The response body is exactly `hello`."),
    );

    let requests = model.requests();
    assert_eq!(requests.len(), 1, "extract is a single judgment leg");
    let request = &requests[0];
    let system = request.system.as_deref().unwrap();
    assert!(system.starts_with("# source.extract"), "extract prompt is the system channel");
    let user = &request.messages[0].content;
    assert!(user.contains("source key `greeting`"), "passed source key is named");
    assert!(user.contains("$SOURCE_DIR"), "binding is mapped onto the prompt's vocabulary");
    assert!(user.contains("extract mines only this source"), "nothing else is reachable");
    let (name, schema) = schema_format(request);
    assert_eq!(name, "evidence");
    assert_eq!(schema, evidence_schema());
    assert_eq!(request.workspace.as_deref(), Some("."), "the source view is lent");
    let tools: Vec<&str> =
        function_tools(request).into_iter().map(|tool| tool.name.as_str()).collect();
    assert_eq!(tools, ["list_docs", "read_doc"], "the reference tools are declared");
}

// An inline `value:` binding lends no workspace: the material rides in
// the user message and the judgment leg gets no filesystem grant.
#[tokio::test]
async fn extract_value_no_lend() {
    let model = Scripted::answering([r#"{"authority":"documentation","claims":[
            {"kind":"requirement","id":"greeting.behaviour","statement":"GET /greeting returns the static string 'hello'."}
        ]}"#]);
    let input = SourceInput::value("greeting", "GET /greeting returns the static string 'hello'.");

    let evidence = Adapter::extract(&model, &ctx(&[]).without_lend(), &input).await.unwrap();

    assert_eq!(evidence.claims.len(), 1);
    let requests = model.requests();
    let request = &requests[0];
    assert_eq!(request.workspace, None, "no lend for an inline value");
    let user = &request.messages[0].content;
    assert!(
        user.contains("GET /greeting returns the static string 'hello'."),
        "the value rides inline"
    );
    assert!(user.contains("no `$SOURCE_DIR` is lent"));
}

// An empty brief fails closed before spending a model call.
#[tokio::test]
async fn extract_empty_value() {
    let model = Scripted::answering(std::iter::empty::<&str>());
    let result =
        Adapter::extract(&model, &ctx(&[]).without_lend(), &SourceInput::value("greeting", "  "))
            .await;

    match result {
        Err(Error::InvalidRequest(detail)) => {
            assert!(detail.contains("empty"), "detail: {detail}");
        }
        other => panic!("expected an empty-brief refusal, got {other:?}"),
    }
    assert!(model.requests().is_empty(), "no model call on an empty brief");
}
