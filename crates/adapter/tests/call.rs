//! Model call tests.

use std::path::Path;

use emery_adapter::types::{Context, Error};
use emery_adapter::{Error as ModelError, Format, ToolCall, judgment};
use emery_prose::registry::Doc;
use omnia_test::guest::{Scripted, function_tools};
use serde::Deserialize;
use serde_json::json;

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct Answer {
    done: bool,
}

// Sorted by path: the registry lookup is a binary search.
const DOCS: &[Doc] = &[
    Doc {
        path: "prompts/extract.md",
        body: "EXTRACT",
    },
    Doc {
        path: "references/depth.md",
        body: "DEPTH",
    },
];

fn ctx<'a>(docs: &'static [Doc], root: &'a Path) -> Context<'a> {
    Context {
        adapter_id: "target:contracts",
        project_root: root,
        docs,
        lend: Some(".".to_string()),
    }
}

fn call(name: &str, arguments: &serde_json::Value) -> ToolCall {
    ToolCall {
        id: format!("call-{name}"),
        name: name.to_string(),
        arguments: arguments.to_string(),
    }
}

#[tokio::test]
async fn assembles_and_parses() {
    let model = Scripted::answering([r#"{"done":true}"#]);

    let answer: Answer = judgment(
        &model,
        &ctx(DOCS, Path::new(".")),
        "SYSTEM".to_string(),
        "USER".to_string(),
        "probe",
        r#"{"type":"object"}"#,
    )
    .await
    .expect("scripted answer deserializes");
    assert_eq!(answer, Answer { done: true });

    let requests = model.requests();
    assert_eq!(requests.len(), 1);
    let request = &requests[0];
    assert_eq!(request.system.as_deref(), Some("SYSTEM"));
    assert_eq!(request.messages[0].content, "USER");
    match &request.format {
        Format::Schema(schema) => {
            assert_eq!(schema.name, "probe");
            assert_eq!(schema.schema, r#"{"type":"object"}"#);
        }
        other => panic!("expected schema format, got {other:?}"),
    }
    assert_eq!(
        request.workspace.as_deref(),
        Some("."),
        "every judgment leg lends the context's workspace path"
    );
    let names: Vec<&str> =
        function_tools(request).into_iter().map(|tool| tool.name.as_str()).collect();
    assert_eq!(names, ["list_docs", "read_doc"], "a docs-carrying judgment declares the tools");
}

// A docs-free judgment declares no tools and stays single-shot.
#[tokio::test]
async fn no_docs_no_tools() {
    let model = Scripted::answering([r#"{"done":true}"#]);

    let _: Answer = judgment(
        &model,
        &ctx(&[], Path::new(".")),
        String::new(),
        "USER".to_string(),
        "probe",
        "{}",
    )
    .await
    .expect("tool-free leg succeeds");

    assert!(model.requests()[0].tools.is_empty());
    assert!(model.exchanges().is_empty());
}

// The closure answers reference tool calls from the embedded corpus:
// canonical JSON objects on success, repairable messages otherwise.
#[tokio::test]
async fn closure_answers_reference_calls() {
    let model = Scripted::answering([r#"{"done":true}"#]).calling(
        0,
        [
            call("list_docs", &json!({})),
            call("read_doc", &json!({"path": "references/depth.md"})),
            call("read_doc", &json!({"path": "references/missing.md"})),
            call("resolve", &json!({})),
        ],
    );

    let answer: Answer = judgment(
        &model,
        &ctx(DOCS, Path::new(".")),
        "SYSTEM".to_string(),
        "USER".to_string(),
        "probe",
        "{}",
    )
    .await
    .expect("the reply follows the answered calls");
    assert_eq!(answer, Answer { done: true });

    let exchanges = model.exchanges();
    assert_eq!(exchanges.len(), 4);
    assert_eq!(
        exchanges[0].outcome,
        Ok(json!({"paths": ["prompts/extract.md", "references/depth.md"]}).to_string())
    );
    assert_eq!(
        exchanges[1].outcome,
        Ok(json!({"path": "references/depth.md", "body": "DEPTH"}).to_string())
    );
    assert!(
        exchanges[2].outcome.as_ref().is_err_and(|err| err.contains("references/missing.md")),
        "an unembedded path is a repairable error: {:?}",
        exchanges[2].outcome
    );
    assert!(
        exchanges[3].outcome.as_ref().is_err_and(|err| err.contains("unknown tool")),
        "an undeclared tool is a repairable error: {:?}",
        exchanges[3].outcome
    );
}

#[tokio::test]
async fn error_mapping() {
    let model = Scripted::new([
        Err(ModelError::InvalidRequest("messages must not be empty".to_string())),
        Ok(emery_adapter::Reply {
            answer: "this is not json".to_string(),
            usage: None,
        }),
    ]);
    let context = ctx(&[], Path::new("."));

    let invalid: Result<Answer, Error> =
        judgment(&model, &context, String::new(), "a".to_string(), "probe", "{}").await;
    assert!(matches!(invalid, Err(Error::InvalidRequest(_))));

    let malformed: Result<Answer, Error> =
        judgment(&model, &context, String::new(), "b".to_string(), "probe", "{}").await;
    match malformed {
        Err(Error::Internal(detail)) => {
            assert!(detail.contains("probe answer did not deserialize"), "detail: {detail}");
        }
        other => panic!("expected internal error, got {other:?}"),
    }
}

// Only answer-tail failures enter the bounded repair loop.
mod repaired {
    use emery_adapter::{MAX_REPAIRS, repaired};

    use super::*;

    fn tail(answer: &str) -> Result<Answer, Error> {
        let parsed: Answer = serde_json::from_str(answer)
            .map_err(|err| Error::Internal(format!("probe answer did not deserialize: {err}")))?;
        if parsed.done {
            Ok(parsed)
        } else {
            Err(Error::Internal("- probe: done must be true".to_string()))
        }
    }

    #[tokio::test]
    async fn repairs_tail_failure() {
        let model = Scripted::answering([r#"{"done":false}"#, r#"{"done":true}"#]);

        let answer = repaired(
            &model,
            &ctx(&[], Path::new(".")),
            "SYSTEM".to_string(),
            "USER".to_string(),
            "probe",
            "{}",
            tail,
        )
        .await
        .expect("repaired answer passes the tail");
        assert_eq!(answer, Answer { done: true });

        let requests = model.requests();
        assert_eq!(requests.len(), 2, "one repair after the failed tail");
        assert_eq!(requests[1].system.as_deref(), Some("SYSTEM"), "system channel is unchanged");
        let repair = &requests[1].messages[0].content;
        assert!(repair.starts_with("USER"), "repair prompt opens with the original request");
        assert!(repair.contains(r#"{"done":false}"#), "and carries the rejected answer");
        assert!(repair.contains("- probe: done must be true"), "and the findings");
    }

    // Repair prompts rebuild from the original request rather than nesting.
    #[tokio::test]
    async fn budget_exhausted() {
        let model = Scripted::answering([r#"{"done":false}"#; 1 + MAX_REPAIRS]);

        let result: Result<Answer, Error> = repaired(
            &model,
            &ctx(&[], Path::new(".")),
            String::new(),
            "USER".to_string(),
            "probe",
            "{}",
            tail,
        )
        .await;
        match result {
            Err(Error::Internal(detail)) => {
                assert!(detail.contains("done must be true"), "detail: {detail}");
            }
            other => panic!("expected the last tail failure, got {other:?}"),
        }

        let requests = model.requests();
        assert_eq!(requests.len(), 1 + MAX_REPAIRS, "initial answer plus the repair budget");
        let last = &requests[requests.len() - 1].messages[0].content;
        assert_eq!(
            last.matches("## Previous answer").count(),
            1,
            "repair prompts rebuild from the original request, never nest"
        );
    }

    // Model failures are not replayed because the request is unchanged.
    #[tokio::test]
    async fn model_failure_not_retried() {
        let model = Scripted::new([Err(ModelError::InvalidRequest(
            "messages must not be empty".to_string(),
        ))]);

        let result: Result<Answer, Error> = repaired(
            &model,
            &ctx(&[], Path::new(".")),
            String::new(),
            "USER".to_string(),
            "probe",
            "{}",
            tail,
        )
        .await;
        assert!(matches!(result, Err(Error::InvalidRequest(_))));
        assert_eq!(model.requests().len(), 1, "a model failure is never replayed");
    }

    // Every repair turn re-declares the reference tools, so the model
    // can keep pulling while it corrects the answer.
    #[tokio::test]
    async fn repair_keeps_tools() {
        let model = Scripted::answering([r#"{"done":false}"#, r#"{"done":true}"#]);

        let _ = repaired(
            &model,
            &ctx(DOCS, Path::new(".")),
            String::new(),
            "USER".to_string(),
            "probe",
            "{}",
            tail,
        )
        .await
        .expect("repaired answer passes the tail");

        for request in model.requests() {
            assert_eq!(function_tools(&request).len(), 2, "both turns declare the tools");
        }
    }
}

// A prepared workspace replaces the default `"."` lend.
#[tokio::test]
async fn lending_overrides_lend() {
    let model = Scripted::answering([r#"{"done":true}"#]);
    let context = ctx(&[], Path::new(".")).lending("/emery-workspaces/ws-1");

    let _: Answer = judgment(&model, &context, String::new(), "USER".to_string(), "probe", "{}")
        .await
        .expect("lent leg succeeds");

    assert_eq!(model.requests()[0].workspace.as_deref(), Some("/emery-workspaces/ws-1"));
}

#[tokio::test]
async fn value_omits_workspace() {
    let model = Scripted::answering([r#"{"done":true}"#]);
    let context = ctx(&[], Path::new(".")).without_lend();

    let _: Answer = judgment(&model, &context, String::new(), "USER".to_string(), "probe", "{}")
        .await
        .expect("value leg succeeds");

    assert_eq!(model.requests()[0].workspace, None);
}
