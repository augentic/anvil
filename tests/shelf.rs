//! The MCP review surface after `specify`: the spec shelf serves the
//! committed generation read-only over the guest HTTP listener, and
//! every other path or method answers the typed refusal (C3).

#![cfg(not(target_arch = "wasm32"))]

mod support;

use emery_transport::http::{DESIGN_URI, GENERATION_URI, SPEC_ROUTE, SPEC_URI};
use omnia_guest::axum::Router;
use omnia_guest::axum::body::{Body, to_bytes};
use omnia_guest::http::{Method, Request, StatusCode, header};
use serde_json::{Value, json};
use support::{Provider, cli_ok, router};
use tower::ServiceExt as _;

const SPEC_ANSWER: &str = include_str!("specify/1-spec.md");
const DESIGN_ANSWER: &str = include_str!("specify/2-design.md");

// Commits one generation through the real `specify` arc, returning
// the provider and the committed generation id.
async fn committed() -> (Provider, String) {
    let workspace = tempfile::TempDir::new_in(env!("CARGO_MANIFEST_DIR")).expect("project tempdir");
    let component = workspace.path().join("source.wasm");
    std::fs::write(&component, b"\0asm-stub").expect("stub wasm");
    let component = component
        .strip_prefix(env!("CARGO_MANIFEST_DIR"))
        .expect("path under project")
        .to_str()
        .expect("utf-8 path");

    let provider = Provider::answering([SPEC_ANSWER, DESIGN_ANSWER]);
    cli_ok(&provider, &["emery", "specify", component]).await;

    let pointer = provider.storage.state("spec/current").expect("current");
    let id = String::from_utf8(pointer).expect("utf-8 pointer").trim().to_string();
    (provider, id)
}

async fn send(router: Router, method: Method, path: &str, body: Body) -> (StatusCode, Vec<u8>) {
    let request = Request::builder()
        .method(method)
        .uri(path)
        .header(header::CONTENT_TYPE, "application/json")
        .body(body)
        .expect("build request");
    let response = router.oneshot(request).await.expect("the router serves the request");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.expect("collect body");
    (status, bytes.to_vec())
}

// Drive one JSON-RPC message through the shelf, returning the parsed reply.
async fn mcp(provider: &Provider, message: &Value) -> Value {
    let listener = emery_transport::http::listener(provider.clone());
    let (status, bytes) =
        send(listener, Method::POST, SPEC_ROUTE, Body::from(message.to_string())).await;
    assert_eq!(status, StatusCode::OK, "{message}");
    serde_json::from_slice(&bytes).expect("JSON-RPC reply")
}

// An HTTP operation surface requires an authenticated ingress design
// (ADR-0002): every command route, every deleted verb path, and every
// unknown path answers the typed refusal on the full listener and the
// bare refusal router alike.
#[tokio::test]
async fn every_route_refuses() {
    let provider = Provider::idle();
    // Derivation from the live inventory prevents new verbs gaining
    // HTTP twins, the spec shelf route included.
    let mut paths: Vec<String> = router(&provider)
        .inventory()
        .iter()
        .map(|route| format!("/{}", route.selector().path().join("/")))
        .collect();
    paths.extend(
        [
            "/",
            "/init",
            "/journal",
            "/plan/status",
            "/plan/execute",
            "/adapter/add",
            "/no/such/path",
        ]
        .map(str::to_string),
    );

    for path in &paths {
        for method in [Method::GET, Method::POST, Method::PUT, Method::DELETE] {
            let routers = [
                emery_transport::http::listener(provider.clone()),
                emery_transport::http::refusal(),
            ];
            for target in routers {
                let (status, bytes) = send(target, method.clone(), path, Body::empty()).await;
                let value: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
                assert_eq!(status, StatusCode::NOT_FOUND, "{method} {path}: {value}");
                assert_eq!(
                    value["error"], "http-surface-disabled",
                    "{method} {path} renders the typed refusal: {value}"
                );
            }
        }
    }
}

// The shelf route itself is JSON-RPC over POST alone.
#[tokio::test]
async fn shelf_post_only() {
    let (provider, _) = committed().await;
    for method in [Method::GET, Method::PUT, Method::DELETE] {
        let listener = emery_transport::http::listener(provider.clone());
        let (status, _) = send(listener, method.clone(), SPEC_ROUTE, Body::empty()).await;
        assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED, "{method} {SPEC_ROUTE}");
    }
}

#[tokio::test]
async fn initialize() {
    let (provider, _) = committed().await;
    let reply = mcp(&provider, &json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize" })).await;
    assert_eq!(reply["result"]["serverInfo"]["name"], "emery-spec");
    assert_eq!(reply["result"]["serverInfo"]["version"], env!("CARGO_PKG_VERSION"));
    assert!(reply["result"]["capabilities"].get("resources").is_some());
}

// The shelf serves the committed generation: the listing names it,
// and every resource read returns the committed bytes.
#[tokio::test]
async fn serves_generation() {
    let (provider, id) = committed().await;
    let spec = provider
        .storage
        .object("spec", &format!("generations/{id}/spec.md"))
        .expect("committed spec.md");
    let design = provider
        .storage
        .object("spec", &format!("generations/{id}/design.md"))
        .expect("committed design.md");

    let reply =
        mcp(&provider, &json!({ "jsonrpc": "2.0", "id": 1, "method": "resources/list" })).await;
    let resources = reply["result"]["resources"].as_array().expect("resource list");
    let uris: Vec<&str> = resources.iter().filter_map(|entry| entry["uri"].as_str()).collect();
    assert_eq!(uris, [SPEC_URI, DESIGN_URI, GENERATION_URI]);
    assert!(
        resources[0]["description"].as_str().expect("description").contains(&id),
        "the listing names the generation: {resources:?}"
    );

    for (uri, mime, body) in [
        (SPEC_URI, "text/markdown", spec.as_slice()),
        (DESIGN_URI, "text/markdown", design.as_slice()),
        (GENERATION_URI, "text/plain", id.as_bytes()),
    ] {
        let reply = mcp(
            &provider,
            &json!({
                "jsonrpc": "2.0", "id": 2, "method": "resources/read",
                "params": { "uri": uri }
            }),
        )
        .await;
        let contents = &reply["result"]["contents"][0];
        assert_eq!(contents["uri"], uri);
        assert_eq!(contents["mimeType"], mime);
        assert_eq!(
            contents["text"].as_str().expect("text").as_bytes(),
            body,
            "{uri} serves the committed bytes"
        );
    }
}

// Read tools mirror the resources for MCP clients without resource support.
#[tokio::test]
async fn tools_mirror_resources() {
    let (provider, id) = committed().await;
    let spec = provider
        .storage
        .object("spec", &format!("generations/{id}/spec.md"))
        .expect("committed spec.md");
    let design = provider
        .storage
        .object("spec", &format!("generations/{id}/design.md"))
        .expect("committed design.md");

    let reply = mcp(&provider, &json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list" })).await;
    let names: Vec<&str> = reply["result"]["tools"]
        .as_array()
        .expect("tool list")
        .iter()
        .filter_map(|tool| tool["name"].as_str())
        .collect();
    assert_eq!(names, ["read_spec", "read_design", "generation"]);

    for (tool, body) in [
        ("read_spec", spec.as_slice()),
        ("read_design", design.as_slice()),
        ("generation", id.as_bytes()),
    ] {
        let reply = mcp(
            &provider,
            &json!({
                "jsonrpc": "2.0", "id": 2, "method": "tools/call",
                "params": { "name": tool }
            }),
        )
        .await;
        assert_eq!(reply["result"]["isError"], false, "{tool}");
        assert_eq!(
            reply["result"]["content"][0]["text"].as_str().expect("text").as_bytes(),
            body,
            "{tool}"
        );
    }
}

// Before any generation commits, the shelf lists nothing and every
// read hints at the mutation path.
#[tokio::test]
async fn empty_store() {
    let provider = Provider::idle();

    let reply =
        mcp(&provider, &json!({ "jsonrpc": "2.0", "id": 1, "method": "resources/list" })).await;
    assert_eq!(reply["result"]["resources"], json!([]));

    let reply = mcp(
        &provider,
        &json!({
            "jsonrpc": "2.0", "id": 2, "method": "resources/read",
            "params": { "uri": SPEC_URI }
        }),
    )
    .await;
    assert_eq!(reply["error"]["code"], -32002, "{reply}");
    let message = reply["error"]["message"].as_str().expect("error message");
    assert!(message.contains("emery specify"), "the error hints at the mutation path: {message}");

    let reply = mcp(
        &provider,
        &json!({
            "jsonrpc": "2.0", "id": 3, "method": "tools/call",
            "params": { "name": "read_spec" }
        }),
    )
    .await;
    assert_eq!(reply["result"]["isError"], true, "{reply}");
}

#[tokio::test]
async fn unknown_resource() {
    let (provider, _) = committed().await;
    let reply = mcp(
        &provider,
        &json!({
            "jsonrpc": "2.0", "id": 1, "method": "resources/read",
            "params": { "uri": "spec://no-such" }
        }),
    )
    .await;
    assert_eq!(reply["error"]["code"], -32002, "{reply}");
}
