//! Wire-contract coverage for `emery plan defer`: argv through the
//! command router → exit 2 with the `plan-deferral-invalid` kebab
//! discriminant on the JSON error envelope, and the exit-0 happy path.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use mock::invoke::run;
use native::{DynModel, Provider, ReferenceMode};
use omnia_guest::api::invoke::Invoker;
use omnia_testkit::model::Harness;
use project::config::Layout;
use project::plan::{AuthorityOverride, Entry, Plan};
use tempfile::TempDir;

fn provider(root: impl Into<PathBuf>) -> Provider {
    let root = root.into();
    let locations = project::handler::Locations::explicit(
        root.join("store"),
        project::handler::CachePlacement::Parent(root.join("project-cache")),
    );
    Provider::new(
        project::handler::ExecutionPaths::new(root, locations),
        DynModel::new(Harness::answering(Vec::<String>::new())),
        mock::catalog(),
        ReferenceMode::Offline,
    )
}

/// Project scaffolded on the mock adapter with one refined slice `a`
/// carrying an open `[unknown]` row and a single-entry plan.
async fn fixture() -> (TempDir, Provider) {
    let project = tempfile::tempdir().expect("tempdir");
    let provider = provider(project.path());
    run::<project::init::handlers::Init, _, _>(
        &provider,
        project::init::handlers::InitInput {
            adapter: Some("mock".to_string()),
            name: Some("demo".to_string()),
            ..Default::default()
        },
    )
    .await
    .expect("init");

    let slice_dir = project.path().join(".emery/slices/a");
    fs::create_dir_all(slice_dir.join("specs")).expect("slice/specs");
    fs::write(
        slice_dir.join("model.yaml"),
        r"requirements:
  - id: REQ-001
    title: reset path not evidenced
    statement: ''
    status: unknown
",
    )
    .expect("model.yaml");
    fs::write(
        slice_dir.join("metadata.yaml"),
        "target: mock\ncreated-at: 2026-01-01T00:00:00Z\ndefined-at: 2026-01-01T00:00:01Z\n",
    )
    .expect("metadata");

    let plan = Plan {
        name: "test".into(),
        sources: BTreeMap::new(),
        entries: vec![Entry {
            name: "a".into(),
            project: None,
            depends_on: vec![],
            sources: vec![],
            context: vec![],
            description: None,
            divergence: None,
            disagreements: Vec::new(),
            authority_override: AuthorityOverride::default(),
            allow_composition_replace: false,
        }],
    };
    plan.save(&Layout::new(project.path()).plan_path()).expect("save plan");
    (project, provider)
}

/// Dispatch `plan defer` argv in JSON format; returns the exit code
/// plus the raw stdout / stderr channels.
async fn defer_json(provider: &Provider, extra: &[&str]) -> (u8, String, String) {
    let router =
        transport::command::router(Invoker::new("emery", provider.clone())).expect("router");
    let mut argv = vec!["emery", "--format", "json", "plan", "defer"];
    argv.extend_from_slice(extra);
    let response = router.execute(argv).await;
    (
        response.exit,
        String::from_utf8(response.stdout).expect("stdout is UTF-8"),
        String::from_utf8(response.stderr).expect("stderr is UTF-8"),
    )
}

fn envelope(stderr: &str) -> serde_json::Value {
    serde_json::from_str(stderr)
        .unwrap_or_else(|err| panic!("stderr is one JSON envelope ({err}): {stderr}"))
}

#[tokio::test]
async fn deferral_invalid_on_wire() {
    let (_project, provider) = fixture().await;

    // Unknown selector.
    let (exit, _, stderr) = defer_json(&provider, &["a/REQ-404", "--reason", "why"]).await;
    assert_eq!(exit, 2);
    let body = envelope(&stderr);
    assert_eq!(body["error"], "plan-deferral-invalid");
    assert_eq!(body["exit-code"], 2);

    // Missing `--reason` on defer is a handler refusal, not a clap
    // usage error — same discriminant.
    let (exit, _, stderr) = defer_json(&provider, &["a/REQ-001"]).await;
    assert_eq!(exit, 2);
    assert_eq!(envelope(&stderr)["error"], "plan-deferral-invalid");
}

#[tokio::test]
async fn defer_and_retract_succeed_on_wire() {
    let (_project, provider) = fixture().await;

    let (exit, stdout, stderr) =
        defer_json(&provider, &["a/REQ-001", "--reason", "deferred to next change"]).await;
    assert_eq!(exit, 0, "{stderr}");
    let body: serde_json::Value = serde_json::from_str(&stdout).expect("stdout JSON");
    assert_eq!(body["action"], "deferred");
    assert_eq!(body["gaps"][0]["slice"], "a");
    assert_eq!(body["gaps"][0]["req"], "REQ-001");
    assert!(
        body["gaps"][0]["requirement-digest"].as_str().is_some_and(|d| d.starts_with("sha256:")),
        "{body}"
    );

    let (exit, stdout, stderr) = defer_json(&provider, &["a/REQ-001", "--retract"]).await;
    assert_eq!(exit, 0, "{stderr}");
    let body: serde_json::Value = serde_json::from_str(&stdout).expect("stdout JSON");
    assert_eq!(body["action"], "retracted");
}
