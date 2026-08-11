//! Wire-contract coverage for the RFC-86 execute validation codes:
//! argv through the command router → exit 2 with the kebab-case
//! discriminant on the JSON `error` envelope (`plan-epoch-stale`;
//! open gaps defer at the gate and never refuse), plus the RFC-86a
//! waiver hard cut (`--waive` is unknown argv; `--gap-policy` is the
//! closed override).

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use jiff::Timestamp;
use mock::invoke::run;
use native::{DynModel, Provider, ReferenceMode};
use omnia_guest::api::invoke::Invoker;
use omnia_testkit::model::Harness;
use project::GapPolicy;
use project::config::Layout;
use project::journal::{ClosedPlanCoverage, Event, EventKind, append_for};
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
/// carrying `model` and a single-entry plan, so `plan execute` reaches
/// the build gate.
async fn fixture(model: &str) -> (TempDir, Provider) {
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
    fs::write(slice_dir.join("model.yaml"), model).expect("model.yaml");
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

/// Dispatch `plan execute` argv in JSON format; returns the exit code
/// and the parsed stderr error envelope.
async fn execute_json(provider: &Provider, extra: &[&str]) -> (u8, serde_json::Value) {
    let router =
        transport::command::router(Invoker::new("emery", provider.clone())).expect("router");
    let mut argv = vec!["emery", "--format", "json", "plan", "execute"];
    argv.extend_from_slice(extra);
    let response = router.execute(argv).await;
    let stderr = String::from_utf8(response.stderr).expect("stderr is UTF-8");
    let envelope = serde_json::from_str(&stderr)
        .unwrap_or_else(|err| panic!("stderr is one JSON envelope ({err}): {stderr}"));
    (response.exit, envelope)
}

const CONFLICT_MODEL: &str = r"requirements:
  - id: REQ-001
    title: contradiction
    statement: ''
    status: conflict
    sources: [intent]
";

#[tokio::test]
async fn waive_is_unknown_argv() {
    // RFC-86a acceptance 9 (hard cut): the per-epoch waiver surface is
    // deleted — `--waive` fails at the grammar (usage error, exit 2)
    // before any dispatch.
    let (_project, provider) = fixture(CONFLICT_MODEL).await;
    let router =
        transport::command::router(Invoker::new("emery", provider.clone())).expect("router");
    let response = router
        .execute(["emery", "plan", "execute", "--waive", "a/REQ-001", "--reason", "why"])
        .await;
    assert_eq!(response.exit, 2);
    let stderr = String::from_utf8_lossy(&response.stderr);
    assert!(stderr.contains("--waive"), "clap names the unknown flag: {stderr}");
}

#[tokio::test]
async fn gap_policy_rejects_values_outside_the_closed_enum() {
    let (_project, provider) = fixture(CONFLICT_MODEL).await;
    let router =
        transport::command::router(Invoker::new("emery", provider.clone())).expect("router");
    let response = router.execute(["emery", "plan", "execute", "--gap-policy", "sometimes"]).await;
    assert_eq!(response.exit, 2);
    let stderr = String::from_utf8_lossy(&response.stderr);
    assert!(stderr.contains("--gap-policy"), "clap names the offending flag: {stderr}");
}

#[tokio::test]
async fn open_gaps_defer_at_gate_on_wire() {
    // The gap-policy gate is gone: an open conflict no longer refuses
    // execute — the gate mints its deferral fact and the run proceeds
    // past the gate (stopping later on the fixture's missing pins).
    let (project, provider) = fixture(CONFLICT_MODEL).await;

    let (exit, envelope) = execute_json(&provider, &[]).await;
    assert_eq!(exit, 2);
    assert_eq!(envelope["error"], "plan-execute-stopped");
    assert_eq!(envelope["exit-code"], 2);

    let minted = project::journal::read_union(Layout::new(project.path()))
        .expect("union")
        .into_iter()
        .any(|event| matches!(event.kind, EventKind::GapDeferred { .. }));
    assert!(minted, "the gate journals the deferral before proceeding");
}

#[tokio::test]
async fn epoch_stale_on_wire() {
    let (project, provider) = fixture(
        r"requirements:
  - id: REQ-001
    title: login works
    statement: ''
    status: agreed
    sources: [intent]
",
    )
    .await;

    // A concurrent writer's later epoch (simulated with a future
    // timestamp) is the newest and no longer covers the live plan.
    let peer_epoch = Event::new(
        Timestamp::from_second(4_102_444_800).expect("timestamp"),
        EventKind::PlanExecuteStarted {
            coverage: ClosedPlanCoverage::ClosedPlan {
                plan_digest:
                    "sha256:0000000000000000000000000000000000000000000000000000000000000000".into(),
                specs: BTreeMap::new(),
                gap_policy: GapPolicy::Strict,
            },
            discovery_digest: None,
        },
    );
    append_for(Layout::new(project.path()), "peer", &[peer_epoch]).expect("peer epoch");

    let (exit, envelope) = execute_json(&provider, &[]).await;
    assert_eq!(exit, 2);
    assert_eq!(envelope["error"], "plan-epoch-stale");
    assert_eq!(envelope["exit-code"], 2);
}
