//! Composed workflow tests over the Milestone F deployment: the workflow
//! guest plus committed adapter guests from the sibling
//! `augentic/specify-adapters` checkout, driven in command mode with the
//! model backend stubbed (RFC-61 Step 4, Milestone F).
//!
//! Three proofs the walking skeleton retired or deferred:
//!
//! - **Merge-leg drain** — a plan whose only slice is already `built`
//!   (seeded natively with mocked seams) drains through the guest's
//!   `plan execute` without a model call, and the merge cadence lands in
//!   the journal on the shared `"."` preopen.
//! - **Link dispatch + pending model future** — `source survey` routes
//!   through `specify:adapter/source` to the *real* committed intent
//!   guest, whose judgment leg awaits `omnia:model/completion`; the stub
//!   backend pends then fails, and the failure comes back as the typed
//!   `seam-dispatch-failed` exit — not a trap.
//! - **Adapter MCP shelves** — each committed adapter guest serves its
//!   own reference shelf on its own `/mcp/<name>` route beside the
//!   workflow guest.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result};
use jiff::Timestamp;
use omnia::{DeploymentBuilder, ExitStatus, Mode};
use serde_json::{Value, json};
use specify_workflow::orchestrate;
use specify_workflow::seam::{Evidence, Lead, MockSourceSeam, MockTargetSeam, WorkingTree};
use specify_workflow::slice::{BuildReport, BuildStatus, SLICES_DIR_NAME, UiSurface};
use tempfile::TempDir;

use crate::common::{self, CacheGuard, Quiet, StubBundle, scoped_cache};

fn now() -> Timestamp {
    "2026-01-02T03:04:05Z".parse().expect("fixed timestamp parses")
}

/// A throw-away project tree the composed deployment mounts at `"."`:
/// `.specify/project.yaml`, the `omnia` adapter component staged at the
/// in-repo development probe path (so target resolution works both
/// natively and inside the guest), and a hermetic project cache pinned
/// beneath the tempdir.
struct Project {
    _tmp: TempDir,
    _cache: CacheGuard,
    root: PathBuf,
}

impl Project {
    fn new() -> Self {
        // Native seeding resolves the target in-process; register the
        // stub describe dispatcher (first registration wins) so resolve
        // never needs a nested wasmtime instantiation here — describe
        // dispatch itself is covered by the engine's adapter suites.
        specify_workflow::adapter::describe::register_describe_runner(|_request| {
            Ok(specify_workflow::adapter::describe::DescribeAnswer::default())
        });
        let tmp = TempDir::new().expect("tempdir");
        let root = tmp.path().to_path_buf();
        let cache = scoped_cache(&root);
        for sub in [format!(".specify/{SLICES_DIR_NAME}"), ".specify/specs".to_string()] {
            fs::create_dir_all(root.join(sub)).expect("mkdir");
        }
        fs::write(root.join(".specify/project.yaml"), "name: demo\nadapter: omnia\nrules: {}\n")
            .expect("write project.yaml");
        // Stage the sibling checkout's release-built omnia component at
        // the resolver's in-repo development probe
        // (`<project>/target/wasm32-wasip2/release/specify_omnia.wasm`),
        // which sits under the `"."` mount so the guest sees it too.
        let dev_dir = root.join("target/wasm32-wasip2/release");
        fs::create_dir_all(&dev_dir).expect("mkdir dev release dir");
        fs::copy(
            common::adapter_component_wasm("target:omnia"),
            dev_dir.join("specify_omnia.wasm"),
        )
        .expect("stage omnia component");
        Self {
            _tmp: tmp,
            _cache: cache,
            root,
        }
    }

    fn layout(&self) -> specify_workflow::config::Layout<'_> {
        specify_workflow::config::Layout::new(&self.root)
    }

    fn seed_plan(&self, content: &str) {
        fs::write(self.root.join("plan.yaml"), content).expect("write plan.yaml");
    }

    fn journal_event_ids(&self) -> Vec<String> {
        let path = self.root.join(".specify/journal.jsonl");
        if !path.exists() {
            return Vec::new();
        }
        fs::read_to_string(path)
            .expect("read journal")
            .lines()
            .filter(|line| !line.is_empty())
            .map(|line| {
                let event: Value = serde_json::from_str(line).expect("journal line is JSON");
                event["event"].as_str().expect("event id").to_string()
            })
            .collect()
    }
}

const APPROVED_PLAN: &str = "\
name: demo
lifecycle: approved
sources:
  intent:
    adapter: intent
    value: \"Build the demo.\"
slices:
  - name: feature-x
    status: pending
    sources:
      - { source: intent, lead: feature-x }
";

/// Seed the project natively to the state the guest picks up: the plan
/// surveyed, and `feature-x` refined + built through mocked seams, so
/// the guest's `plan execute` has exactly one model-free merge leg left.
async fn seed_built_slice(project: &Project) {
    let leads = vec![Lead {
        lead: "feature-x".to_string(),
        synopsis: "Operator intent for feature-x.".to_string(),
        topics: vec!["intent".to_string()],
    }];
    let survey_seam = MockSourceSeam::scripted([Ok(leads)], []);
    orchestrate::survey_all(&survey_seam, project.layout(), now()).await.expect("seed survey");

    let model =
        specify_guest_model::MockModel::answering([
            Box::leak(synthesis_response().into_boxed_str()) as &'static str,
        ]);
    let sources = MockSourceSeam::scripted(
        [],
        [Ok(Evidence {
            authority: specify_model::evidence::AuthorityClass::Intent,
            claims: vec![json!({
                "kind": "intent",
                "id": "greeting.fix",
                "statement": "Fix the greeting.",
            })],
        })],
    );
    let targets = MockTargetSeam::scripted(
        [Ok("Shape guidance.".to_string())],
        [Ok(BuildReport {
            version: 1,
            slice: "feature-x".to_string(),
            target: "omnia@1.0.0".to_string(),
            status: BuildStatus::Success,
            findings: vec![],
            outputs: vec![],
            ui_surface: Some(UiSurface { screens: 0 }),
        })],
    );

    orchestrate::refine(
        &model,
        &sources,
        &targets,
        project.layout(),
        now(),
        "feature-x",
        "omnia@1.0.0",
    )
    .await
    .expect("seed refine");
    let tree = WorkingTree {
        base: "live".to_string(),
        subpath: None,
    };
    orchestrate::build(&targets, project.layout(), now(), "feature-x", &[], tree)
        .await
        .expect("seed build");
}

/// A schema-valid synthesis response for `feature-x`: one requirement
/// citing the intent claim, a satisfying task, and the artifacts `slice
/// validate` requires (the workflow crate's execute-test fixture).
fn synthesis_response() -> String {
    serde_json::to_string(&json!({
        "version": 1,
        "kind": "response",
        "slice": "feature-x",
        "model": {
            "requirements": [{
                "title": "greeting behaves as intended",
                "domain": "greeting",
                "claims": [{ "source": "intent", "id": "greeting.fix", "kind": "intent" }],
                "statement": "The greeting surface behaves as the operator intends.",
                "scenarios": ["Intended behaviour observed"]
            }],
            "tasks": [
                { "id": "TASK-001", "text": "Implement the greeting change.", "satisfies": ["REQ-001"] }
            ]
        },
        "artifacts": {
            "proposal": "# feature-x\n\n## Why\n\nThe operator asked for it.\n\n## Domains\n\n- greeting — the affected surface\n\n## Non-goals\n\n- Nothing else.\n",
            "design": "# Design\n\nHow feature-x lands.\n",
            "tasks": "# Tasks\n\n## Implementation\n\n- [ ] 1.1 Implement the change (TASK-001)\n",
            "specs": [{ "domain": "greeting", "content": "## greeting\nAgent prose body.\n" }]
        }
    }))
    .expect("response serialises")
}

// Drive one command-mode run of the composed deployment (workflow guest +
// the given committed adapter guests, `"."` mounted at `mount`) with the
// given guest argv, over the stubbed model backend.
async fn run_composed(mount: &Path, adapters: &[&str], args: &[&str]) -> Result<ExitStatus> {
    let manifest = common::composed_manifest(mount, adapters)?;
    let builder = DeploymentBuilder::new()
        .config(manifest.path().to_path_buf())
        .mode(Mode::Command)
        .args(args.iter().map(ToString::to_string).collect::<Vec<_>>());
    omnia::run::<StubBundle, Quiet>(builder).await
}

// The merge-leg drain: a natively seeded built slice drains through the
// guest's `plan execute` — Gate 1 passes, `plan next` claims the entry,
// the merge folds the slice into the baseline, and the loop exits
// drained with exit 0. Model-free end to end: the stub backend fails any
// completion, so a clean exit also proves no judgment leg fired.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn execute_drains_merge_leg() -> Result<()> {
    let project = Project::new();
    project.seed_plan(APPROVED_PLAN);
    seed_built_slice(&project).await;
    let seeded = project.journal_event_ids().len();

    let status =
        run_composed(&project.root, &["source:intent", "target:omnia"], &["plan", "execute"])
            .await?;
    assert_eq!(status.code(), 0, "the guest execute loop drains the plan");

    // Journal-append assertion over the preopen: the guest appended the
    // claim + merge cadence to the journal the native seed started.
    let ids = project.journal_event_ids();
    assert_eq!(
        &ids[seeded..],
        [
            "plan.entry.advanced",
            "slice.merge.started",
            "slice.merge.commit-skipped",
            "slice.archive.created",
            "slice.merge.succeeded",
        ],
        "the guest merge leg appends exactly the claim + merge cadence"
    );

    // Merge is the only writer of per-entry `done`; the baseline and the
    // archive prove the fold ran against the shared mount.
    let plan = fs::read_to_string(project.root.join("plan.yaml")).expect("read plan.yaml");
    assert!(plan.contains("status: done"), "the entry is stamped done:\n{plan}");
    // The archive dir is stamped with the guest's own clock date, so
    // match by slice suffix rather than a fixed day.
    let archived = fs::read_dir(project.root.join(".specify/archive"))
        .expect("archive dir exists")
        .filter_map(Result::ok)
        .any(|entry| entry.file_name().to_string_lossy().ends_with("-feature-x"));
    assert!(archived, "the slice tree is archived");
    assert!(
        project.root.join(".specify/specs/greeting/spec.md").is_file(),
        "the baseline spec landed"
    );
    assert!(
        !project.root.join(".specify/guest.lock").exists(),
        "the guest marker is released on the clean exit"
    );
    Ok(())
}

// Link dispatch to a real committed adapter guest, surviving a pending
// model future: `source survey intent` routes through
// `specify:adapter/source` to the intent guest, whose judgment leg
// awaits `omnia:model/completion`. The stub backend parks the future and
// then fails, so the WIT error variant must come back across two guest
// boundaries as the typed `seam-dispatch-failed` failure envelope (exit
// 1) — not a trap.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn survey_dispatch_survives_pending_model() -> Result<()> {
    let project = Project::new();
    project.seed_plan(APPROVED_PLAN);

    let status =
        run_composed(&project.root, &["source:intent"], &["source", "survey", "intent"]).await?;
    assert_eq!(status.code(), 1, "the seam failure maps to the generic-failure exit");

    // The orchestrator journalled the dispatch before it failed — the
    // journal write proves the guest's `"."` preopen is the same tree the
    // host seeded.
    let ids = project.journal_event_ids();
    assert_eq!(
        ids.last().map(String::as_str),
        Some("source.execution.agent"),
        "the survey dispatch is journalled before the seam failure: {ids:?}"
    );
    Ok(())
}

// POST one JSON-RPC message to a route and parse the reply.
async fn post(runtime: &omnia::Runtime<StubBundle>, route: &str, message: &Value) -> Result<Value> {
    let response = omnia_testkit::http::post_json(runtime, route, message.to_string()).await?;
    assert!(response.status().is_success(), "MCP POST replies 2xx: {}", response.status());
    serde_json::from_slice(response.body()).context("MCP reply is JSON")
}

// Each committed adapter guest serves its own MCP reference shelf on its
// own route beside the workflow guest — the deployment surface the
// spawned cursor-agent reads through `SPECIFY_<ADAPTER>_MCP_URL`.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn adapter_shelves() -> Result<()> {
    let mount = TempDir::new()?;
    let runtime =
        common::composed_runtime(mount.path(), &["source:intent", "target:omnia"]).await?;

    for (route, adapter) in [("/mcp/intent", "intent"), ("/mcp/omnia", "omnia")] {
        let init = post(
            &runtime,
            route,
            &json!({
                "jsonrpc": "2.0", "id": 1, "method": "initialize",
                "params": { "protocolVersion": "2025-06-18" }
            }),
        )
        .await?;
        let name = init["result"]["serverInfo"]["name"].as_str().unwrap_or_default();
        assert!(
            name.contains(&format!("specify-{adapter}")),
            "{route} identifies the {adapter} shelf: {init}"
        );
    }
    Ok(())
}
