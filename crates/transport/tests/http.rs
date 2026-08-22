//! Wire coverage for the non-MCP HTTP refusal.

use omnia_guest::axum::body::{Body, to_bytes};
use omnia_guest::http::{Method, Request, StatusCode};
use tower::ServiceExt as _;

async fn send(method: Method, path: &str) -> (StatusCode, serde_json::Value) {
    let request =
        Request::builder().method(method).uri(path).body(Body::empty()).expect("build request");
    let response = emery_transport::http::refusal()
        .oneshot(request)
        .await
        .expect("refusal serves the request");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.expect("collect body");
    let value = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, value)
}

#[tokio::test]
async fn every_route_refuses() {
    let paths = [
        "/",
        "/init",
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
            let (status, value) = send(method.clone(), path).await;
            assert_eq!(status, StatusCode::NOT_FOUND, "{method} {path}: {value}");
            assert_eq!(
                value["error"], "http-surface-disabled",
                "{method} {path} renders the typed refusal: {value}"
            );
        }
    }
}
