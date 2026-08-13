//! Transport-level coverage of Emery's shared HTTP router.

use std::fs;
use std::path::Path;

use mock::session::Session;
use omnia_guest::api::invoke::Invoker;
use omnia_guest::axum::Router;
use omnia_guest::axum::body::{Body, to_bytes};
use omnia_guest::http::{Method, Request, StatusCode};
use tower::ServiceExt as _;

/// The HTTP router over an initialised scripted session — routing
/// coverage only; no test dispatches judgment or an adapter seam.
fn router(project: &Session) -> Router {
    transport::http::router(Invoker::new("emery", project.provider().clone())).into_axum()
}

async fn send(router: Router, request: Request<Body>) -> (StatusCode, serde_json::Value) {
    let response = router.oneshot(request).await.expect("router serves the request");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.expect("collect body");
    let value = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, value)
}

/// Stage a minimal two-entry `plan.yaml` in the change home.
fn stage_plan(root: &Path) {
    let path = root.join(".emery/change/plan.yaml");
    fs::create_dir_all(path.parent().expect("parent")).expect("change home");
    fs::write(
        &path,
        "name: demo\ntargets:\n  default:\n    adapter: emery:mock@0.0.0\n    locator: \".\"\n    cid: sha256:0000000000000000000000000000000000000000000000000000000000000000\nslices:\n  - name: first\n    target: default\n  - name: second\n    target: default\n",
    )
        .expect("stage plan.yaml");
}

mod routing {
    use super::*;

    #[tokio::test]
    async fn get_json_body() {
        let project = Session::scripted("mock", Vec::new());
        let request = Request::builder()
            .method(Method::GET)
            .uri("/journal")
            .body(Body::empty())
            .expect("build request");
        let (status, value) = send(router(&project), request).await;
        assert_eq!(status, StatusCode::OK, "journal show serves: {value}");
        assert_eq!(value["count"], 0, "an empty journal projects zero events: {value}");
    }

    #[tokio::test]
    async fn post_body_reaches() {
        let project = Session::scripted("mock", Vec::new());
        let request = Request::builder()
            .method(Method::POST)
            .uri("/archive/prune")
            .header(omnia_guest::http::header::CONTENT_TYPE, "application/json")
            .body(Body::from(r#"{"keep":1,"dry_run":true}"#))
            .expect("build request");
        let (status, value) = send(router(&project), request).await;
        assert_eq!(status, StatusCode::OK, "prune lands: {value}");
    }

    #[tokio::test]
    async fn path_param_reaches() {
        let project = Session::scripted("mock", Vec::new());
        stage_plan(project.root());
        let request = Request::builder()
            .method(Method::POST)
            .uri("/plan/second/remove")
            .body(Body::empty())
            .expect("build request");
        let (status, value) = send(router(&project), request).await;
        assert_eq!(status, StatusCode::OK, "remove lands: {value}");
        let plan =
            fs::read_to_string(project.root().join(".emery/change/plan.yaml")).expect("plan.yaml");
        assert!(!plan.contains("name: second"), "the remove landed:\n{plan}");
    }
}

mod errors {
    use super::*;

    #[tokio::test]
    async fn taxonomy_failure_envelope() {
        let project = Session::scripted("mock", Vec::new());
        let request = Request::builder()
            .method(Method::POST)
            .uri("/archive/prune")
            .body(Body::empty())
            .expect("build request");
        let (status, value) = send(router(&project), request).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "argument failure is 422: {value}");
    }

    #[tokio::test]
    async fn field_unprocessable() {
        let project = Session::scripted("mock", Vec::new());
        let request = Request::builder()
            .method(Method::POST)
            .uri("/adapter/add")
            .body(Body::from("{}"))
            .expect("build request");
        let (status, value) = send(router(&project), request).await;
        assert_eq!(
            status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "a body missing `component` is refused at extraction"
        );
        assert_eq!(
            value["error"], "invalid-request",
            "decode failures render the Emery envelope: {value}"
        );
        assert!(
            value["message"].as_str().is_some_and(|message| message.contains("component")),
            "the decode message names the missing field: {value}"
        );
    }
}
