//! WASM boundary smoke: host the built `specify.wasm` workflow guest
//! with the combined adapter component bound at both
//! `source:fixture` and `target:fixture`, then drive a short scripted
//! path through fresh command-mode deployments — the same hosting
//! shape as the shipped binary, with only the model backend swapped
//! for a colocated script.
//!
//! Ownership is the component boundary only: combined-component loading
//! and WIT linking, metadata dispatch on both axes (init resolves
//! `target:fixture`, author resolves `source:fixture`), source and
//! target operation dispatch, the WIT error lift (a `fail-survey`
//! identity failing typed across the seam), model-host invocation, and
//! writes through the project and `/specify-cache` preopens. The
//! scripted `author → approve → execute` path is the vehicle that
//! reaches those seams — not a second workflow matrix. Workflow
//! behaviour beyond the boundary lives in the cheaper native suites
//! under `crates/change/tests/`.

#![cfg(not(target_arch = "wasm32"))]

use std::collections::VecDeque;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::{Context as _, Result};
use omnia::wasmtime_wasi::ResourceTable;
use omnia::{
    Backend as _, Backends, Deployment, DeploymentBuilder, FutureResult, HasHttp, Mode, Runtime,
    StoreCtx, Wiring, run,
};
use omnia_wasi_http::{HttpDefault, WasiHttp, WasiHttpCtxView};
use omnia_wasi_model::{Answer, HasModel, Request, ToolHost, WasiModel, WasiModelCtx};
use omnia_wasi_otel::{HasOtel, OtelDefault, WasiOtel, WasiOtelCtx};
use serde::Serialize;
use serde_json::{Value, json};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn hosts_boundary() -> Result<()> {
    let tmp = tempfile::tempdir().context("creating the trial workspace")?;
    fs::create_dir_all(tmp.path().join(".specify-cache")).context("creating the cache mount")?;
    fs::create_dir_all(tmp.path().join(".specify-store")).context("creating the store mount")?;
    let workspace = tmp.path().canonicalize().context("resolving the workspace root")?;
    fs::copy(adapter_wasm(), workspace.join("fixture.wasm")).context("staging fixture.wasm")?;
    // The failing identity resolves through the project component
    // cache (only init's own adapter is mirrored automatically).
    fs::create_dir_all(workspace.join(".specify-cache/components"))
        .context("creating the component cache")?;
    fs::copy(adapter_wasm(), workspace.join(".specify-cache/components/fixture-fail-survey.wasm"))
        .context("staging the failing identity")?;
    let manifest_path = workspace.join("omnia.toml");
    fs::write(&manifest_path, manifest(&workspace).render())
        .context("writing the deployment manifest")?;

    script(vec![grouping_answer(), synthesis_answer()]);

    let code = specify(&manifest_path, &["init", "./fixture.wasm", "--name", "wasm"]).await?;
    assert_eq!(code, 0, "wasm init failed");

    // Writable `/specify-cache` preopen: init mirrored the operator's
    // local component and recorded its provenance through it.
    assert!(
        workspace.join(".specify-cache/components/fixture.wasm").is_file(),
        "init mirrors the adapter through the writable cache preopen"
    );
    assert!(
        workspace.join(".specify-cache/components/component-meta.yaml").is_file(),
        "init records component provenance through the writable cache preopen"
    );

    // The WIT error lift across the component seam: the same artifact
    // bound under a `fail-survey` identity dispatches, fails typed in
    // the guest, and surfaces as a non-zero exit — before any scripted
    // answer is consumed.
    let failing: &[&str] =
        &["plan", "author", "wasm", "--source", "main=fixture-fail-survey:value:hello"];
    let code = specify(&manifest_path, failing).await?;
    assert_ne!(code, 0, "a fixture survey failure must fail plan author");
    assert_eq!(
        ANSWERS.lock().expect("the answer script is never poisoned").len(),
        2,
        "the failing survey must abort before the reconcile leg"
    );
    // Authoring creates the plan skeleton before surveying; clear the
    // aborted run's leftover so the real author starts fresh.
    fs::remove_file(workspace.join("plan.yaml")).context("clearing the aborted plan")?;

    // Vehicle path: reach source dispatch, model-host judgment, and
    // target build through the hosted guest. Assert boundary effects
    // only — not drained-loop / baseline-merge workflow outcomes.
    let steps: [&[&str]; 3] = [
        &["plan", "author", "wasm", "--source", "main=fixture:value:hello"],
        &["plan", "transition", "wasm", "approved"],
        &["plan", "execute"],
    ];
    for argv in steps {
        let code = specify(&manifest_path, argv).await?;
        assert_eq!(code, 0, "wasm step `specify {}` failed", argv.join(" "));
    }

    // Target build dispatch: the fixture's observable artifact was
    // written through the guest's own project preopen.
    let built = fs::read_to_string(workspace.join("fixture-build/greeting.md"))
        .context("reading the fixture build output")?;
    assert!(!built.trim().is_empty(), "fixture build output is empty");

    // Model-host invocation: both judgment legs (reconcile, synthesis)
    // consumed their scripted answer through the WASI model host.
    let remaining = ANSWERS.lock().expect("the answer script is never poisoned").len();
    assert_eq!(remaining, 0, "{remaining} scripted answers were never requested");
    Ok(())
}

/// Drive one `specify` invocation through a fresh command-mode
/// deployment over `manifest`, returning the guest's exit code.
async fn specify(manifest: &Path, argv: &[&str]) -> Result<i32> {
    eprintln!("==> specify {}", argv.join(" "));
    let argv: Vec<String> = argv.iter().map(|&arg| arg.to_owned()).collect();
    let status = run::<ScriptedBundle, Quiet>(
        DeploymentBuilder::new().config(manifest.to_path_buf()).args(argv).mode(Mode::Command),
    )
    .await
    .context("hosting the wasm deployment")?;
    Ok(status.code())
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..")
}

fn workflow_wasm() -> PathBuf {
    guest_wasm("specify.wasm")
}

fn adapter_wasm() -> PathBuf {
    guest_wasm("adapter.wasm")
}

fn guest_wasm(relative: &str) -> PathBuf {
    // Honor a redirected target dir (`CARGO_TARGET_DIR`) so the smoke
    // hosts the guests the current cargo invocation actually built.
    let target_dir = std::env::var_os("CARGO_TARGET_DIR")
        .map_or_else(|| repo_root().join("target"), PathBuf::from);
    let path = target_dir.join("wasm32-wasip2/debug").join(relative);
    assert!(
        path.is_file(),
        "guest `{relative}` not found at {}; run `cargo make guests` in harness/",
        path.display()
    );
    path
}

// --- deployment manifest -------------------------------------------------
//
// The minimal typed rendering of the TOML shape the shipped binary
// consumes through `specify run --config`: the workflow guest linked
// against both adapter dispatch interfaces, the combined fixture
// component bound once per axis, and the three writable preopens
// (project root, per-project cache, global adapter store).

fn manifest(workspace: &Path) -> Manifest {
    let links = ["specify:adapter/source@0.1.0", "specify:adapter/target@0.1.0"].map(str::to_owned);
    Manifest {
        guest: vec![
            Guest::new("workflow", &workflow_wasm(), links.to_vec()),
            Guest::new("source:fixture", &adapter_wasm(), Vec::new()),
            Guest::new("target:fixture", &adapter_wasm(), Vec::new()),
            // The same artifact under a failing identity, for the
            // error-lift step.
            Guest::new("source:fixture-fail-survey", &adapter_wasm(), Vec::new()),
        ],
        mount: vec![
            Mount::new(".", workspace, true),
            Mount::new("/specify-cache", &workspace.join(".specify-cache"), true),
            Mount::new("/specify-store", &workspace.join(".specify-store"), true),
        ],
        transport: Transport {
            default: "in-process".to_owned(),
        },
    }
}

#[derive(Debug, Serialize)]
struct Manifest {
    guest: Vec<Guest>,
    mount: Vec<Mount>,
    transport: Transport,
}

impl Manifest {
    fn render(&self) -> String {
        toml::to_string(self).expect("the manifest shape serialises as TOML")
    }
}

#[derive(Debug, Serialize)]
struct Guest {
    id: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    link: Vec<String>,
    source: GuestSource,
}

impl Guest {
    fn new(id: &str, path: &Path, link: Vec<String>) -> Self {
        Self {
            id: id.to_owned(),
            link,
            source: GuestSource {
                path: path.display().to_string(),
            },
        }
    }
}

#[derive(Debug, Serialize)]
struct GuestSource {
    path: String,
}

#[derive(Debug, Serialize)]
struct Mount {
    name: String,
    path: String,
    writable: bool,
}

impl Mount {
    fn new(name: &str, path: &Path, writable: bool) -> Self {
        Self {
            name: name.to_owned(),
            path: path.display().to_string(),
            writable,
        }
    }
}

#[derive(Debug, Serialize)]
struct Transport {
    default: String,
}

// --- scripted model backend ----------------------------------------------

/// The colocated deterministic answer script, in request order.
/// `omnia::Backends::connect` takes no arguments, so the test parks
/// the answers here before driving; one deployment drives at a time
/// per process.
static ANSWERS: Mutex<VecDeque<Value>> = Mutex::new(VecDeque::new());

fn script(answers: Vec<Value>) {
    *ANSWERS.lock().expect("the answer script is never poisoned") = answers.into();
}

/// FIFO scripted model: each completion pops the next parked answer.
#[derive(Clone, Copy, Debug)]
struct Scripted;

impl WasiModelCtx for Scripted {
    fn complete(&self, _request: Request, _tool_host: Arc<dyn ToolHost>) -> FutureResult<Answer> {
        let next = ANSWERS.lock().expect("the answer script is never poisoned").pop_front();
        Box::pin(async move {
            let value = next.context("the scripted model ran out of answers")?;
            Ok(Answer {
                value,
                usage: None,
                transcript: None,
            })
        })
    }
}

/// Scripted bundle: the same host set the shipped binary's
/// `omnia::runtime!` binds (`WasiHttp: HttpDefault`, `WasiOtel:
/// OtelDefault`, `WasiModel: <backend>`), with the cursor model
/// backend swapped for the FIFO script.
#[derive(Clone)]
struct ScriptedBundle {
    http: HttpDefault,
    otel: OtelDefault,
    model: Scripted,
}

impl std::fmt::Debug for ScriptedBundle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScriptedBundle").finish_non_exhaustive()
    }
}

impl Backends for ScriptedBundle {
    async fn connect() -> Result<Self> {
        let (http, otel) = tokio::try_join!(HttpDefault::connect(), OtelDefault::connect())?;
        Ok(Self {
            http,
            otel,
            model: Scripted,
        })
    }
}

impl HasHttp for ScriptedBundle {
    fn http_view<'a>(&'a mut self, table: &'a mut ResourceTable) -> WasiHttpCtxView<'a> {
        self.http.as_view(table)
    }
}

impl HasOtel for ScriptedBundle {
    fn otel_ctx(&mut self) -> &mut dyn WasiOtelCtx {
        &mut self.otel
    }
}

impl HasModel for ScriptedBundle {
    fn model_ctx(&mut self) -> &mut dyn WasiModelCtx {
        &mut self.model
    }
}

/// Scripted wiring: hosts linked, no trigger servers.
#[derive(Debug)]
struct Quiet;

impl Wiring<ScriptedBundle> for Quiet {
    fn link(deployment: &mut Deployment<StoreCtx<ScriptedBundle>>) -> Result<()> {
        deployment.host::<WasiHttp, ScriptedBundle>()?;
        deployment.host::<WasiOtel, ScriptedBundle>()?;
        deployment.host::<WasiModel, ScriptedBundle>()?;
        Ok(())
    }

    async fn serve(_runtime: &Runtime<ScriptedBundle>) -> Result<()> {
        Ok(())
    }
}

// --- colocated judgment answers ------------------------------------------
//
// The fixture's minimal profile: one `greeting` lead groups into one
// slice, and the synthesis cites the single `greeting.behaviour` claim
// the fixture source extracted.

fn grouping_answer() -> Value {
    json!({
        "version": 1,
        "kind": "response",
        "slices": [{
            "name": "greeting",
            "sources": [{ "source": "main", "lead": "greeting" }],
            "rationale": "One fixture lead, one slice."
        }],
        "gate": {
            "change": "## Intent\n\nCharacterise the greeting service.\n\n## Scope\n\nOne slice.",
            "discovery-summary": "Sources: 1. Leads: 1.",
            "discovery-source-inventory": "| key | adapter | binding |\n|---|---|---|\n| main | fixture | \"hello\" |"
        }
    })
}

fn synthesis_answer() -> Value {
    json!({
        "version": 1,
        "kind": "response",
        "slice": "greeting",
        "model": {
            "requirements": [{
                "title": "greeting returns the static string",
                "domain": "greeting",
                "claims": [{ "source": "main", "id": "greeting.behaviour", "kind": "requirement" }],
                "statement": "GET /greeting returns the static string 'hello'.",
                "scenarios": ["A request to /greeting receives 'hello'"]
            }],
            "tasks": [
                { "id": "TASK-001", "text": "Implement the greeting endpoint.", "satisfies": ["REQ-001"] }
            ]
        },
        "artifacts": {
            "proposal": "# greeting\n\n## Why\n\nThe fixture source surfaced it.\n\n## Domains\n\n- greeting — the affected surface\n\n## Non-goals\n\n- Nothing else.\n",
            "design": "# Design\n\nHow the greeting slice lands.\n",
            "tasks": "# Tasks\n\n## Implementation\n\n- [ ] 1.1 Implement the endpoint (TASK-001)\n",
            "specs": [{ "domain": "greeting", "content": "## greeting\nAgent prose body.\n" }]
        }
    })
}
