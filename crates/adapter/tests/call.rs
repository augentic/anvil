//! The shared judgment-call helpers: request assembly, grant and lend
//! wiring, error mapping, and the bounded source answer-tail repair
//! loop.

use std::path::Path;

use adapter::seam::{Context, Error, WorkingTree};
use adapter::{Error as ModelError, Format, judgment};
use serde::Deserialize;
use testkit::{Harness, mcp_grants};

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct Answer {
    done: bool,
}

const fn ctx<'a>(mcp_url: Option<&'a str>, root: &'a Path) -> Context<'a> {
    Context {
        adapter_id: "target:contracts",
        project_root: root,
        mcp_url,
    }
}

#[tokio::test]
async fn assembles_and_parses() {
    let model = Harness::answering([r#"{"done":true}"#]);

    let answer: Answer = judgment(
        &model,
        &ctx(Some("http://references/mcp"), Path::new(".")),
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
    assert!(request.lend_workspace, "every judgment leg lends the workspace");
    let grants = mcp_grants(request);
    assert_eq!(grants.len(), 1);
    assert_eq!(grants[0].name, "contracts-references", "grant named after the adapter");
    assert_eq!(grants[0].url, "http://references/mcp");
}

// Without a resolved MCP URL the leg runs grant-free rather than failing.
#[tokio::test]
async fn no_mcp_no_grant() {
    let model = Harness::answering([r#"{"done":true}"#]);

    let _: Answer = judgment(
        &model,
        &ctx(None, Path::new(".")),
        String::new(),
        "USER".to_string(),
        "probe",
        "{}",
    )
    .await
    .expect("grant-free leg succeeds");

    assert!(model.requests()[0].tools.is_empty());
}

#[tokio::test]
async fn error_mapping() {
    let model = Harness::scripted([
        Err(ModelError::InvalidRequest("messages must not be empty".to_string())),
        Ok(adapter::Reply {
            answer: "this is not json".to_string(),
            usage: None,
        }),
    ]);
    let context = ctx(None, Path::new("."));

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

// The bounded repair loop around source answer tails: tail failures
// re-prompt with the findings inlined; everything else returns
// immediately.
mod repaired {
    use adapter::{MAX_REPAIRS, repaired};

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

    // A tail failure triggers one repair; the repair prompt carries the
    // original request, the rejected answer, and the findings.
    #[tokio::test]
    async fn repairs_tail_failure() {
        let model = Harness::answering([r#"{"done":false}"#, r#"{"done":true}"#]);

        let answer = repaired(
            &model,
            &ctx(None, Path::new(".")),
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

    // After the initial answer plus MAX_REPAIRS failed repairs the last
    // tail failure surfaces; repair prompts rebuild from the original
    // request rather than nesting.
    #[tokio::test]
    async fn budget_exhausted() {
        let model = Harness::answering([r#"{"done":false}"#; 1 + MAX_REPAIRS]);

        let result: Result<Answer, Error> = repaired(
            &model,
            &ctx(None, Path::new(".")),
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

    // A model failure returns immediately: the request did not change,
    // so replaying it is pointless.
    #[tokio::test]
    async fn model_failure_not_retried() {
        let model = Harness::scripted([Err(ModelError::InvalidRequest(
            "messages must not be empty".to_string(),
        ))]);

        let result: Result<Answer, Error> = repaired(
            &model,
            &ctx(None, Path::new(".")),
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
}

#[test]
fn tree_root() {
    let context = ctx(None, Path::new("/mnt"));
    let bare = WorkingTree {
        base: "rev-1".to_string(),
        subpath: None,
    };
    let scoped = WorkingTree {
        base: "rev-1".to_string(),
        subpath: Some("proj".to_string()),
    };
    assert_eq!(context.tree_root(&bare), Path::new("/mnt"));
    assert_eq!(context.tree_root(&scoped), Path::new("/mnt/proj"));
}
