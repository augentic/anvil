//! Model-free composed coverage for the workflow guest's WASM-only boundary.

#![cfg(not(target_arch = "wasm32"))]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result};
use omnia::wasmtime_wasi::ResourceTable;
use omnia::{
    Backend as _, Deployment, DeploymentBuilder, ExitStatus, HasHttp, Mode, Runtime, StoreCtx,
    Wiring, run,
};
use omnia_testkit::temp_manifest;
use omnia_wasi_http::{HttpDefault, WasiHttp, WasiHttpCtxView};
use omnia_wasi_model::{HasModel, ModelDefault, WasiModel, WasiModelCtx};
use scenario::grade::{Execution, StepResult};
use scenario::{ModelBackend, Outcome, Runtime as ScenarioRuntime, Scenario};

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
    model: ModelDefault,
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
            model: ModelDefault::from_dir("__no_composed_replay_fixtures__")?,
        })
    }
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

fn workflow_wasm() -> PathBuf {
    guest_wasm("specify.wasm")
}

fn echo_target_wasm() -> PathBuf {
    guest_wasm("examples/echo_target.wasm")
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
