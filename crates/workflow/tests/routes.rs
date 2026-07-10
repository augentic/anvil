//! Transport-level coverage of the HTTP seam: assemble a route table
//! over a tempdir-anchored provider (the same `route::get` / `post`
//! constructor lines the shims' hand-written tables use) and drive
//! real requests through it, asserting path/query/body extraction
//! reaches the same `Handler` impls the argv transport drives and
//! that failures ride the taxonomy → status projection.
//!
//! The provider's judgment seams (`Model`, `SourceSeam`, `TargetSeam`)
//! are erroring stubs — they exist to satisfy the route constructors'
//! bounds; the deterministic verbs exercised here never touch them.

use std::fs;
use std::path::{Path, PathBuf};

use omnia_guest::api::{Client, route};
use omnia_guest::axum::Router;
use omnia_guest::axum::body::{Body, to_bytes};
use omnia_guest::http::{Method, Request, StatusCode};
use omnia_guest::model;
use tempfile::TempDir;
use tower::ServiceExt as _;
use workflow::seam::{self, Evidence, Input, Lead, SourceSeam, TargetSeam, WorkingTree};
use workflow::slice::BuildReport;
use workflow::{registry, slice};

/// An initialised throw-away project tree. The tempdir stays owned by
/// the test — the provider moves into the router's state and drops
/// with it, so it must not carry the directory's lifetime.
struct Project {
    _tmp: TempDir,
    root: PathBuf,
}

impl Project {
    /// An initialised project (`.specify/project.yaml` present).
    fn initialised() -> Self {
        let tmp = TempDir::new().expect("tempdir");
        let root = tmp.path().canonicalize().expect("canonical tempdir");
        fs::create_dir_all(root.join(".specify")).expect("mkdir .specify");
        fs::write(root.join(".specify/project.yaml"), "name: demo\nadapter: demo\nrules: {}\n")
            .expect("write project.yaml");
        Self { _tmp: tmp, root }
    }

    /// A route table anchored at this project's root, covering the
    /// verbs this suite drives — the same constructor lines the shims'
    /// full tables carry.
    fn router(&self) -> Router {
        let provider = Provider {
            root: self.root.clone(),
        };
        Router::new()
            .route("/registry/validate", route::get::<registry::verbs::Validate, Provider>())
            .route("/registry", route::post::<registry::verbs::Add, Provider>())
            .route("/registry/{name}/remove", route::post::<registry::verbs::Remove, Provider>())
            .route("/archive/prune", route::post::<slice::verbs::Prune, Provider>())
            .with_state(Client::new("specify").provider(provider))
    }
}

/// Root-anchored provider satisfying the router's full seam bound.
struct Provider {
    root: PathBuf,
}

impl workflow::verb::Anchor for Provider {
    fn project_root(&self) -> &Path {
        &self.root
    }
}

impl omnia_guest::Model for Provider {
    async fn create(&self, _: model::Request) -> Result<model::Reply, model::Error> {
        Err(model::Error::Backend("no model behind the router test".to_string()))
    }
}

impl SourceSeam for Provider {
    async fn survey(&self, _: String) -> Result<Vec<Lead>, seam::Error> {
        Err(seam::Error::Internal("no source seam behind the router test".to_string()))
    }

    async fn extract(&self, _: String, _: Lead) -> Result<Evidence, seam::Error> {
        Err(seam::Error::Internal("no source seam behind the router test".to_string()))
    }
}

impl TargetSeam for Provider {
    async fn guidance(&self, _: String) -> Result<String, seam::Error> {
        Err(seam::Error::Internal("no target seam behind the router test".to_string()))
    }

    async fn build(
        &self, _: String, _: String, _: Vec<Input>, _: WorkingTree,
    ) -> Result<BuildReport, seam::Error> {
        Err(seam::Error::Internal("no target seam behind the router test".to_string()))
    }
}

/// Send one request, returning the status and the parsed JSON body
/// (`Null` when the body is not JSON).
async fn send(router: Router, request: Request<Body>) -> (StatusCode, serde_json::Value) {
    let response = router.oneshot(request).await.expect("router serves the request");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.expect("collect body");
    let value = serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null);
    (status, value)
}

/// Stage a one-project `registry.yaml` at the project root.
fn stage_registry(root: &Path) {
    fs::write(
        root.join("registry.yaml"),
        "version: 1\nprojects:\n  - name: alpha\n    url: git@example.com:org/alpha.git\n",
    )
    .expect("stage registry.yaml");
}

#[tokio::test]
async fn get_serves_json_body() {
    let project = Project::initialised();
    stage_registry(&project.root);
    let request = Request::builder()
        .method(Method::GET)
        .uri("/registry/validate")
        .body(Body::empty())
        .expect("build request");
    let (status, value) = send(project.router(), request).await;
    assert_eq!(status, StatusCode::OK, "staged catalogue validates: {value}");
}

#[tokio::test]
async fn post_body_reaches_handler() {
    let project = Project::initialised();
    let request = Request::builder()
        .method(Method::POST)
        .uri("/registry")
        .header(omnia_guest::http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(r#"{"name":"alpha","url":"git@example.com:org/alpha.git"}"#))
        .expect("build request");
    let (status, value) = send(project.router(), request).await;
    assert_eq!(status, StatusCode::OK, "add lands: {value}");
    let registry = fs::read_to_string(project.root.join("registry.yaml")).expect("registry.yaml");
    assert!(registry.contains("name: alpha"), "the add landed:\n{registry}");
}

#[tokio::test]
async fn path_param_reaches_handler() {
    let project = Project::initialised();
    stage_registry(&project.root);
    let request = Request::builder()
        .method(Method::POST)
        .uri("/registry/alpha/remove")
        .body(Body::empty())
        .expect("build request");
    let (status, value) = send(project.router(), request).await;
    assert_eq!(status, StatusCode::OK, "remove lands: {value}");
    let registry = fs::read_to_string(project.root.join("registry.yaml")).expect("registry.yaml");
    assert!(!registry.contains("name: alpha"), "the remove landed:\n{registry}");
}

#[tokio::test]
async fn taxonomy_failure_maps_to_error_envelope() {
    // A bound-less prune refuses with `Error::Argument` — the argv
    // transport exits 2; here the same failure must surface as the
    // 422 error envelope from the one taxonomy → status projection.
    let project = Project::initialised();
    let request = Request::builder()
        .method(Method::POST)
        .uri("/archive/prune")
        .body(Body::empty())
        .expect("build request");
    let (status, value) = send(project.router(), request).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "argument failure is 422: {value}");
}

#[tokio::test]
async fn missing_required_field_is_bad_request() {
    let project = Project::initialised();
    let request = Request::builder()
        .method(Method::POST)
        .uri("/registry")
        .body(Body::from(r#"{"name":"alpha"}"#))
        .expect("build request");
    let (status, _) = send(project.router(), request).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "a body missing `url` is refused at extraction");
}
