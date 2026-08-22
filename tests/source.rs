//! In-process `specify` → `show` journey over scripted capabilities.

#![cfg(not(target_arch = "wasm32"))]

#[path = "../crates/engine/tests/support/storage.rs"]
mod storage;

use std::fs;
use std::future::Future;
use std::sync::Arc;

use emery_adapter::seam::{
    Authority, Backing, Claim, ClaimKind, Evidence, SourceInput, SourceMetadata,
};
use emery_adapter::{DispatchError, Source};
use emery_transport::command;
use omnia_guest::Model;
use omnia_guest::api::command::CommandResponse;
use omnia_guest::api::invoke::Invoker;
use omnia_guest::model::{Error, Reply, Request};
use omnia_testkit::model::{Harness, Scripted};
use serde_json::{Map, Value};
use storage::Memory;

const SPEC_ANSWER: &str = include_str!("source/1-spec.md");
const DESIGN_ANSWER: &str = include_str!("source/2-design.md");

#[tokio::test]
async fn gen_spec() {
    // Only the operator-supplied component touches the filesystem; engine state
    // stays in scripted storage, avoiding chdir and a `.emery/` tree.
    let workspace = tempfile::tempdir().expect("tempdir");
    let component = workspace.path().join("source.wasm");
    fs::write(&component, b"\0asm-stub").expect("stub wasm");

    let provider = Provider {
        model: Harness::answering([SPEC_ANSWER, DESIGN_ANSWER, SPEC_ANSWER, DESIGN_ANSWER]),
        storage: Arc::new(Memory::default()),
    };

    let component = component.to_str().expect("utf-8 path");

    // One `specify` ensures, mirrors, extracts, and commits — no prior verb.
    cli_exec(&provider, &["emery", "specify", component]).await;
    assert_eq!(
        provider.storage.object("adapters", "source.wasm").as_deref(),
        Some(b"\0asm-stub".as_slice()),
        "the component is mirrored into the cache container"
    );
    assert!(provider.storage.state("project.yaml").is_none(), "no project record exists");
    let pointer = provider.storage.state("spec/current").expect("current");
    let id = String::from_utf8(pointer).expect("utf-8 pointer").trim().to_string();
    let spec =
        provider.storage.object("spec", &format!("generations/{id}/spec.md")).expect("spec.md");
    assert!(String::from_utf8_lossy(&spec).contains("[unknown]"));
    let design =
        provider.storage.object("spec", &format!("generations/{id}/design.md")).expect("design.md");
    assert!(!design.is_empty());

    // Review is `show`: text stdout is the stored document, byte for byte.
    let shown = cli_exec(&provider, &["emery", "show", "spec"]).await;
    assert_eq!(shown.stdout, spec, "show renders the committed spec.md alone");
    let shown = cli_exec(&provider, &["emery", "show", "design"]).await;
    assert_eq!(shown.stdout, design, "show renders the committed design.md alone");

    // The MCP shelf projects the same committed bytes over the listener.
    let read = mcp_read(&provider, emery_transport::http::SPEC_URI).await;
    assert_eq!(read.as_bytes(), spec, "the shelf serves the committed spec.md");
    let read = mcp_read(&provider, emery_transport::http::GENERATION_URI).await;
    assert_eq!(read, id, "the shelf serves the current generation id");

    let resp = cli_exec(&provider, &["emery", "specify", component]).await;
    let stdout = String::from_utf8_lossy(&resp.stdout);
    assert!(stdout.contains("none (byte-stable)"), "{stdout}");

    provider.model.assert_exhausted();
}

// Reads one shelf resource over the guest HTTP router, layer-2 style.
async fn mcp_read(provider: &Provider, uri: &str) -> String {
    use tower::ServiceExt as _;

    let message = serde_json::json!({
        "jsonrpc": "2.0", "id": 1, "method": "resources/read",
        "params": { "uri": uri }
    });
    let request = omnia_guest::http::Request::builder()
        .method(omnia_guest::http::Method::POST)
        .uri(emery_transport::http::SPEC_ROUTE)
        .body(omnia_guest::axum::body::Body::from(message.to_string()))
        .expect("build request");
    let response = emery_transport::http::listener(provider.clone())
        .oneshot(request)
        .await
        .expect("the listener serves the request");
    assert_eq!(response.status(), omnia_guest::http::StatusCode::OK, "{uri}");
    let bytes = omnia_guest::axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("collect body");
    let reply: Value = serde_json::from_slice(&bytes).expect("JSON-RPC reply");
    reply["result"]["contents"][0]["text"].as_str().unwrap_or_else(|| panic!("{reply}")).to_string()
}

async fn cli_exec(provider: &Provider, argv: &[&str]) -> CommandResponse {
    let router = command::router(Invoker::new("emery", provider.clone())).expect("command grammar");
    let resp = router.execute(argv.iter().copied()).await;
    assert_eq!(resp.exit, 0, "{}", String::from_utf8_lossy(&resp.stderr));
    resp
}

#[derive(Clone, Debug)]
struct Provider {
    model: Harness<Scripted>,
    storage: Arc<Memory>,
}

crate::scripted_storage!(Provider, storage);

impl Model for Provider {
    async fn create(&self, request: Request) -> Result<Reply, Error> {
        self.model.create(request).await
    }
}

impl Source for Provider {
    fn extract(
        &self, _id: &str, _input: &SourceInput,
    ) -> impl Future<Output = Result<Evidence, DispatchError>> + Send {
        let statement = "GET /greeting returns the static string 'hello'.";
        let mut extras = Map::new();
        extras.insert("statement".to_string(), Value::String(statement.to_string()));

        std::future::ready(Ok(Evidence {
            authority: Authority::Documentation,
            claims: vec![Claim {
                kind: ClaimKind::Requirement,
                id: Some("greeting.behaviour".to_string()),
                path: None,
                synopsis: Some("Greeting behaviour".to_string()),
                backing: Some(Backing::Payload(statement.to_string())),
                extras,
            }],
        }))
    }

    fn metadata(&self, _id: &str) -> SourceMetadata {
        SourceMetadata { emery_floor: None }
    }
}
