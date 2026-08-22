//! Wire coverage for the guest HTTP surface: the spec shelf and the refusal.

#[path = "../../engine/tests/support/storage.rs"]
mod storage;

use std::sync::Arc;

use emery_engine::home::{CURRENT_KEY, SPEC_CONTAINER, SpecSet};
use emery_transport::http::{DESIGN_URI, GENERATION_URI, SPEC_ROUTE, SPEC_URI};
use omnia_guest::axum::Router;
use omnia_guest::axum::body::{Body, to_bytes};
use omnia_guest::http::{Method, Request, StatusCode, header};
use serde_json::{Value, json};
use storage::Memory;
use tower::ServiceExt as _;

const SPEC_BODY: &str = "# Spec\n\nOne requirement.\n";
const DESIGN_BODY: &str = "# Design\n\nOne decision.\n";

// The listener needs only the storage capabilities.
#[derive(Clone, Debug, Default)]
struct Store {
    storage: Arc<Memory>,
}

crate::scripted_storage!(Store, storage);

// Seeds storage with one committed generation, exactly as `Home::commit` writes it.
fn seeded() -> (Store, String) {
    let provider = Store::default();
    let set = SpecSet {
        spec: SPEC_BODY.to_string(),
        design: DESIGN_BODY.to_string(),
    };
    let id = set.id();
    for (name, body) in set.files() {
        provider.storage.insert_object(
            SPEC_CONTAINER,
            &format!("generations/{id}/{name}"),
            body.as_bytes(),
        );
    }
    provider.storage.insert_state(CURRENT_KEY, format!("{id}\n").as_bytes());
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
async fn mcp(provider: &Store, message: &Value) -> Value {
    let listener = emery_transport::http::listener(provider.clone());
    let (status, bytes) =
        send(listener, Method::POST, SPEC_ROUTE, Body::from(message.to_string())).await;
    assert_eq!(status, StatusCode::OK, "{message}");
    serde_json::from_slice(&bytes).expect("JSON-RPC reply")
}

#[tokio::test]
async fn every_route_refuses() {
    let paths = [
        "/",
        "/init",
        "/specify",
        "/show",
        "/journal",
        "/plan/status",
        "/plan/execute",
        "/plan/demo/drop",
        "/system/review",
        "/archive/prune",
        "/adapter/add",
        "/no/such/path",
    ];
    for path in paths {
        for method in [Method::GET, Method::POST, Method::PUT, Method::DELETE] {
            // The full listener and the bare refusal refuse identically.
            let routers = [
                emery_transport::http::listener(Store::default()),
                emery_transport::http::refusal(),
            ];
            for router in routers {
                let (status, bytes) = send(router, method.clone(), path, Body::empty()).await;
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

#[tokio::test]
async fn shelf_post_only() {
    let (provider, _) = seeded();
    for method in [Method::GET, Method::PUT, Method::DELETE] {
        let listener = emery_transport::http::listener(provider.clone());
        let (status, _) = send(listener, method.clone(), SPEC_ROUTE, Body::empty()).await;
        assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED, "{method} {SPEC_ROUTE}");
    }
}

#[tokio::test]
async fn shelf_initialize() {
    let (provider, _) = seeded();
    let reply = mcp(&provider, &json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize" })).await;
    assert_eq!(reply["result"]["serverInfo"]["name"], "emery-spec");
    assert_eq!(reply["result"]["serverInfo"]["version"], env!("CARGO_PKG_VERSION"));
    assert!(reply["result"]["capabilities"].get("resources").is_some());
}

#[tokio::test]
async fn shelf_serves_generation() {
    let (provider, id) = seeded();

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
        (SPEC_URI, "text/markdown", SPEC_BODY),
        (DESIGN_URI, "text/markdown", DESIGN_BODY),
        (GENERATION_URI, "text/plain", id.as_str()),
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
        assert_eq!(contents["text"], body, "{uri}");
    }
}

#[tokio::test]
async fn shelf_tools_mirror_resources() {
    let (provider, id) = seeded();

    let reply = mcp(&provider, &json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list" })).await;
    let names: Vec<&str> = reply["result"]["tools"]
        .as_array()
        .expect("tool list")
        .iter()
        .filter_map(|tool| tool["name"].as_str())
        .collect();
    assert_eq!(names, ["read_spec", "read_design", "generation"]);

    for (tool, body) in
        [("read_spec", SPEC_BODY), ("read_design", DESIGN_BODY), ("generation", id.as_str())]
    {
        let reply = mcp(
            &provider,
            &json!({
                "jsonrpc": "2.0", "id": 2, "method": "tools/call",
                "params": { "name": tool }
            }),
        )
        .await;
        assert_eq!(reply["result"]["isError"], false, "{tool}");
        assert_eq!(reply["result"]["content"][0]["text"], body, "{tool}");
    }
}

#[tokio::test]
async fn shelf_empty_store() {
    let provider = Store::default();

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
async fn shelf_unknown_resource() {
    let (provider, _) = seeded();
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
