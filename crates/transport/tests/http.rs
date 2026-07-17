//! Transport-level coverage of Specify's shared HTTP router.

use std::fs;
use std::path::Path;

use fixture::session::Session;
use omnia_guest::api::invoke::Invoker;
use omnia_guest::axum::Router;
use omnia_guest::axum::body::{Body, to_bytes};
use omnia_guest::http::{Method, Request, StatusCode};
use tower::ServiceExt as _;

/// The HTTP router over an initialised scripted session — routing
/// coverage only; no test dispatches judgment or an adapter seam.
fn router(project: &Session) -> Router {
    transport::http::router(Invoker::new("specify", project.provider().clone())).into_axum()
}

async fn send(router: Router, request: Request<Body>) -> (StatusCode, serde_json::Value) {
    let response = router.oneshot(request).await.expect("router serves the request");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.expect("collect body");
    let value = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, value)
}

fn stage_registry(root: &Path) {
    fs::write(
        root.join("registry.yaml"),
        "version: 1\nprojects:\n  - name: alpha\n    url: git@example.com:org/alpha.git\n",
    )
    .expect("stage registry.yaml");
}

mod routing {
    use super::*;

    #[tokio::test]
    async fn get_json_body() {
        let project = Session::scripted("fixture", Vec::new());
        stage_registry(project.root());
        let request = Request::builder()
            .method(Method::GET)
            .uri("/registry/validate")
            .body(Body::empty())
            .expect("build request");
        let (status, value) = send(router(&project), request).await;
        assert_eq!(status, StatusCode::OK, "staged catalogue validates: {value}");
    }

    #[tokio::test]
    async fn post_body_reaches_operation() {
        let project = Session::scripted("fixture", Vec::new());
        let request = Request::builder()
            .method(Method::POST)
            .uri("/registry")
            .header(omnia_guest::http::header::CONTENT_TYPE, "application/json")
            .body(Body::from(r#"{"name":"alpha","url":"git@example.com:org/alpha.git"}"#))
            .expect("build request");
        let (status, value) = send(router(&project), request).await;
        assert_eq!(status, StatusCode::OK, "add lands: {value}");
        let registry =
            fs::read_to_string(project.root().join("registry.yaml")).expect("registry.yaml");
        assert!(registry.contains("name: alpha"), "the add landed:\n{registry}");
    }

    #[tokio::test]
    async fn path_param_reaches_operation() {
        let project = Session::scripted("fixture", Vec::new());
        stage_registry(project.root());
        let request = Request::builder()
            .method(Method::POST)
            .uri("/registry/alpha/remove")
            .body(Body::empty())
            .expect("build request");
        let (status, value) = send(router(&project), request).await;
        assert_eq!(status, StatusCode::OK, "remove lands: {value}");
        let registry =
            fs::read_to_string(project.root().join("registry.yaml")).expect("registry.yaml");
        assert!(!registry.contains("name: alpha"), "the remove landed:\n{registry}");
    }
}

mod errors {
    use super::*;

    #[tokio::test]
    async fn taxonomy_failure_envelope() {
        let project = Session::scripted("fixture", Vec::new());
        let request = Request::builder()
            .method(Method::POST)
            .uri("/archive/prune")
            .body(Body::empty())
            .expect("build request");
        let (status, value) = send(router(&project), request).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "argument failure is 422: {value}");
    }

    #[tokio::test]
    async fn missing_field_unprocessable() {
        let project = Session::scripted("fixture", Vec::new());
        let request = Request::builder()
            .method(Method::POST)
            .uri("/registry")
            .body(Body::from(r#"{"name":"alpha"}"#))
            .expect("build request");
        let (status, value) = send(router(&project), request).await;
        assert_eq!(
            status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "a body missing `url` is refused at extraction"
        );
        assert_eq!(
            value["error"], "invalid-request",
            "decode failures render the Specify envelope: {value}"
        );
        assert!(
            value["message"].as_str().is_some_and(|message| message.contains("url")),
            "the decode message names the missing field: {value}"
        );
    }
}
