//! Model-free composed coverage for the workflow guest's WASM-only boundary.

#![cfg(not(target_arch = "wasm32"))]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::{Context as _, Result};
use omnia::wasmtime_wasi::ResourceTable;
use omnia::{
    Backend as _, Deployment, DeploymentBuilder, ExitStatus, HasHttp, Mode, Runtime, StoreCtx,
    Wiring, run,
};
use omnia_testkit::temp_manifest;
use omnia_wasi_http::{HttpDefault, WasiHttp, WasiHttpCtxView};
use omnia_wasi_model::{Answer, FutureResult, HasModel, ToolHost, WasiModel, WasiModelCtx};
use scenario::grade::{Execution, StepResult};
use scenario::{ModelBackend, Outcome, Runtime as ScenarioRuntime, Scenario};
use serde_json::json;

const SOURCE_INTERFACE: &str = "specify:adapter/source@0.1.0";
const TARGET_INTERFACE: &str = "specify:adapter/target@0.1.0";

struct Hosts;

impl Wiring<Bundle> for Hosts {
    fn link(deployment: &mut Deployment<StoreCtx<Bundle>>) -> Result<()> {
        deployment.host::<WasiHttp, Bundle>()?;
        deployment.host::<WasiModel, Bundle>()?;
        Ok(())
    }

    async fn serve(_runtime: &Runtime<Bundle>) -> Result<()> {
        Ok(())
    }
}

#[derive(Clone)]
struct Bundle {
    http: HttpDefault,
    model: RecordingModel,
}

impl std::fmt::Debug for Bundle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Bundle").finish_non_exhaustive()
    }
}

impl omnia::Backends for Bundle {
    async fn connect() -> Result<Self> {
        Ok(Self {
            http: HttpDefault::connect().await?,
            model: RecordingModel,
        })
    }
}

#[derive(Clone, Copy, Debug)]
struct RecordingModel;

static REQUEST_INDEX: AtomicUsize = AtomicUsize::new(0);

impl WasiModelCtx for RecordingModel {
    fn complete(
        &self, request: omnia_wasi_model::Request, _tool_host: Arc<dyn ToolHost>,
    ) -> FutureResult<Answer> {
        let index = REQUEST_INDEX.fetch_add(1, Ordering::SeqCst);
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../quality/fixtures/replay/composed-loop")
            .join(format!("request-{index}.json"));
        std::fs::create_dir_all(path.parent().expect("request fixture parent"))
            .expect("create request fixture directory");
        std::fs::write(
            &path,
            serde_json::to_vec_pretty(&replay_value(&request)).expect("serialise replay request"),
        )
        .expect("write replay request");
        let value = if index == 0 {
            json!({
                "version": 1,
                "kind": "response",
                "slices": [{
                    "name": "echo",
                    "sources": [{ "source": "echo", "lead": "echo" }],
                    "rationale": "The echo source emits one lead."
                }],
                "gate": {
                    "change": "## Echo\n\nExercise the composed workflow boundary.",
                    "discovery-summary": "Sources: 1. Leads: 1.",
                    "discovery-source-inventory": "| key | adapter | binding |\n|---|---|---|\n| echo | echo-source | \"hello\" |"
                }
            })
        } else {
            json!({
                "version": 1,
                "kind": "response",
                "slice": "echo",
                "model": {
                    "requirements": [{
                        "title": "echo survives the workflow",
                        "domain": "echo",
                        "claims": [{ "source": "echo", "id": "echo.excerpt.001", "kind": "excerpt" }],
                        "statement": "The echo lead is represented in the baseline.",
                        "scenarios": ["Echo baseline merged"]
                    }],
                    "tasks": [{
                        "id": "TASK-001",
                        "text": "Exercise the echo target.",
                        "satisfies": ["REQ-001"]
                    }]
                },
                "artifacts": {
                    "proposal": "# Echo\n\n## Why\n\nExercise the composed boundary.\n\n## Domains\n\n- echo — fixture domain\n\n## Non-goals\n\n- Production behaviour.\n",
                    "design": "# Design\n\nUse the deterministic echo target.\n",
                    "tasks": "# Tasks\n\n## Implementation\n\n- [ ] 1.1 Exercise the echo target (TASK-001)\n",
                    "specs": [{
                        "domain": "echo",
                        "content": "## Echo\n\nThe echo lead survives the workflow.\n"
                    }]
                }
            })
        };
        Box::pin(std::future::ready(Ok(Answer {
            value,
            usage: None,
            transcript: None,
        })))
    }
}

fn replay_value(request: &omnia_wasi_model::Request) -> serde_json::Value {
    let format = match &request.format {
        omnia_wasi_model::Format::Text => json!({ "kind": "text" }),
        omnia_wasi_model::Format::Json => json!({ "kind": "json" }),
        omnia_wasi_model::Format::Schema(schema) => json!({
            "kind": "schema",
            "schema": { "name": schema.name, "schema": schema.schema },
        }),
    };
    let tools = request
        .tools
        .iter()
        .map(|tool| match tool {
            omnia_wasi_model::Tool::Function(function) => json!({
                "function": {
                    "name": function.name,
                    "description": function.description,
                    "parameters": function.parameters,
                },
            }),
            omnia_wasi_model::Tool::Mcp(mcp) => json!({
                "mcp": { "name": mcp.name, "tools": mcp.tools, "url": mcp.url },
            }),
        })
        .collect::<Vec<_>>();
    json!({
        "model": request.model,
        "system": request.system,
        "messages": request.messages.iter().map(|message| json!({
            "role": message.role.to_string(),
            "content": message.content,
        })).collect::<Vec<_>>(),
        "generation": request.generation.as_ref().map(|generation| json!({
            "temperature": generation.temperature,
            "top_p": generation.top_p,
            "max_tokens": generation.max_tokens,
            "stop": generation.stop,
            "seed": generation.seed,
            "effort": generation.effort.map(|effort| effort.to_string()),
        })),
        "format": format,
        "tools": tools,
        "grants": {
            "references": request.grants.references,
            "verify": request.grants.verify,
        },
    })
}

impl HasHttp for Bundle {
    fn http_view<'a>(&'a mut self, table: &'a mut ResourceTable) -> WasiHttpCtxView<'a> {
        self.http.as_view(table)
    }
}

impl HasModel for Bundle {
    fn model_ctx(&mut self) -> &mut dyn WasiModelCtx {
        &mut self.model
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn init_dispatches_and_writes_preopens() -> Result<()> {
    let scenario = Scenario::load(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../quality/scenarios/composed-init.yaml"),
    )
    .context("loading composed scenario")?;
    let profile = scenario
        .profiles
        .iter()
        .find(|profile| profile.id == "wasm-replay")
        .context("composed scenario declares wasm-replay")?;
    assert_eq!(profile.runtime, ScenarioRuntime::Wasm);
    assert_eq!(profile.model, ModelBackend::Replay);
    assert_eq!(
        scenario.workflow[0].run,
        "specify init ./echo-target.wasm --name composed-smoke --scaffold-only"
    );

    let project = tempfile::tempdir().context("creating project mount")?;
    let cache = tempfile::tempdir().context("creating cache mount")?;
    let adapter = project.path().join("echo-target.wasm");
    std::fs::copy(echo_target_wasm(), &adapter).context("staging target fixture")?;

    let manifest = temp_manifest(&manifest(project.path(), cache.path()))?;
    let status = run::<Bundle, Hosts>(
        DeploymentBuilder::new()
            .config(manifest.path().to_path_buf())
            .args([
                "init".to_string(),
                "./echo-target.wasm".to_string(),
                "--name".to_string(),
                "composed-smoke".to_string(),
                "--scaffold-only".to_string(),
            ])
            .mode(Mode::Command),
    )
    .await
    .context("running composed workflow command")?;

    assert_eq!(status, ExitStatus::SUCCESS);
    let project_yaml = std::fs::read_to_string(project.path().join(".specify/project.yaml"))
        .context("reading project.yaml written through the project preopen")?;
    assert!(project_yaml.contains("name: composed-smoke"), "{project_yaml}");
    assert!(project_yaml.contains("adapter: file://"), "{project_yaml}");
    assert!(
        cache.path().join("components/echo-target.wasm").is_file(),
        "init mirrors the dispatched adapter through the writable cache preopen"
    );
    assert!(
        cache.path().join("components/component-meta.yaml").is_file(),
        "init records component provenance through the writable cache preopen"
    );
    let execution = Execution::new(
        project.path(),
        BTreeMap::from([(
            "init".to_string(),
            StepResult {
                exit_code: 0,
                stdout: String::new(),
                stderr: String::new(),
            },
        )]),
    );
    let assertions = scenario::grade::hard(&scenario, &execution);
    assert!(
        assertions.iter().all(|assertion| assertion.outcome == Outcome::Pass),
        "{assertions:?}"
    );
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn replay_drives_full_loop() -> Result<()> {
    REQUEST_INDEX.store(0, Ordering::SeqCst);
    let scenario = Scenario::load(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../quality/scenarios/composed-loop.yaml"),
    )
    .context("loading composed loop scenario")?;
    let project = tempfile::tempdir().context("creating project mount")?;
    let cache = tempfile::tempdir().context("creating cache mount")?;
    std::fs::copy(echo_target_wasm(), project.path().join("echo-target.wasm"))
        .context("staging target fixture")?;
    let manifest = temp_manifest(&loop_manifest(project.path(), cache.path()))?;

    let commands = [
        ("init", vec!["init", "./echo-target.wasm", "--name", "composed-loop", "--scaffold-only"]),
        (
            "author",
            vec!["plan", "author", "composed-loop", "--source", "echo=echo-source:value:hello"],
        ),
        ("approve", vec!["plan", "transition", "composed-loop", "approved"]),
        ("execute", vec!["plan", "execute"]),
    ];
    let mut steps = BTreeMap::new();
    for (id, args) in commands {
        let status = run::<Bundle, Hosts>(
            DeploymentBuilder::new()
                .config(manifest.path().to_path_buf())
                .args(args.into_iter().map(str::to_owned).collect::<Vec<_>>())
                .mode(Mode::Command),
        )
        .await
        .with_context(|| format!("running composed `{id}` command"))?;
        steps.insert(
            id.to_owned(),
            StepResult {
                exit_code: if status == ExitStatus::SUCCESS { 0 } else { 1 },
                stdout: String::new(),
                stderr: String::new(),
            },
        );
        assert_eq!(status, ExitStatus::SUCCESS, "composed step `{id}` failed");
    }

    let assertions = scenario::grade::hard(&scenario, &Execution::new(project.path(), steps));
    assert!(
        assertions.iter().all(|assertion| assertion.outcome == Outcome::Pass),
        "{assertions:?}"
    );
    assert!(
        project.path().join(".specify/specs/echo/spec.md").is_file(),
        "merge writes the baseline spec"
    );
    Ok(())
}

fn manifest(project: &Path, cache: &Path) -> String {
    format!(
        r#"[[guest]]
id = "workflow"
source.path = "{}"
link = ["{SOURCE_INTERFACE}", "{TARGET_INTERFACE}"]

[[guest]]
id = "target:echo-target"
source.path = "{}"

[[mount]]
name = "."
path = "{}"
writable = true

[[mount]]
name = "/specify-cache"
path = "{}"
writable = true

[transport]
default = "in-process"
"#,
        workflow_wasm().display(),
        echo_target_wasm().display(),
        project.display(),
        cache.display(),
    )
}

fn loop_manifest(project: &Path, cache: &Path) -> String {
    format!(
        r#"[[guest]]
id = "workflow"
source.path = "{}"
link = ["{SOURCE_INTERFACE}", "{TARGET_INTERFACE}"]

[[guest]]
id = "source:echo-source"
source.path = "{}"

[[guest]]
id = "target:echo-target"
source.path = "{}"

[[mount]]
name = "."
path = "{}"
writable = true

[[mount]]
name = "/specify-cache"
path = "{}"
writable = true

[transport]
default = "in-process"
"#,
        workflow_wasm().display(),
        echo_source_wasm().display(),
        echo_target_wasm().display(),
        project.display(),
        cache.display(),
    )
}

fn workflow_wasm() -> PathBuf {
    guest_wasm("specify.wasm")
}

fn echo_target_wasm() -> PathBuf {
    guest_wasm("examples/echo_target.wasm")
}

fn echo_source_wasm() -> PathBuf {
    guest_wasm("examples/echo_source.wasm")
}

fn guest_wasm(relative: &str) -> PathBuf {
    let path = target_dir().join("wasm32-wasip2/debug").join(relative);
    assert!(
        path.is_file(),
        "guest `{relative}` not found at {}; run `cargo make build-composed-guests` in harness/",
        path.display()
    );
    path
}

fn target_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("manifest is <workspace>/harness/composed")
        .join("target")
}
