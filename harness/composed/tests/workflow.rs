//! Replay-backed composed coverage for the workflow guest's WASM-only boundary.

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
use scenario::{AssertionId, ModelBackend, Outcome, Runtime as ScenarioRuntime, Scenario};

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
            model: ModelDefault::from_dir(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../quality/fixtures/replay/composed-loop"),
            )?,
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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn replay_drives_full_loop() -> Result<()> {
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
                exit_code: i32::from(status != ExitStatus::SUCCESS),
                stdout: String::new(),
                stderr: String::new(),
            },
        );
        assert_eq!(status, ExitStatus::SUCCESS, "composed step `{id}` failed");
    }

    let assertions = grade_composed(
        scenario::grade::hard(&scenario, &Execution::new(project.path(), steps)),
        project.path(),
    )?;
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

fn grade_composed(
    mut assertions: Vec<scenario::AssertionResult>, project: &Path,
) -> Result<Vec<scenario::AssertionResult>> {
    let plan =
        std::fs::read_to_string(project.join("plan.yaml")).context("reading composed loop plan")?;
    let baseline = project.join(".specify/specs/echo/spec.md");
    for assertion in &mut assertions {
        let (passed, evidence) = match assertion.id {
            AssertionId::ComposedPlanDrained => (
                plan.contains("status: done")
                    && !plan.contains("status: pending")
                    && !plan.contains("status: in-progress"),
                "plan.yaml has one done entry and no pending entry".to_owned(),
            ),
            AssertionId::ComposedArtifactsComplete => (
                std::fs::read_to_string(&baseline)
                    .is_ok_and(|spec| spec.contains("REQ-001") && spec.contains("Sources: echo")),
                "merged baseline contains the projected requirement and provenance".to_owned(),
            ),
            AssertionId::ComposedBaselineMergeVisible => {
                (baseline.is_file(), baseline.display().to_string())
            }
            _ => continue,
        };
        assertion.outcome = if passed { Outcome::Pass } else { Outcome::Fail };
        assertion.evidence = Some(evidence);
        assertion.detail = (!passed).then(|| "composed profile evaluator failed".to_owned());
    }
    Ok(assertions)
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
