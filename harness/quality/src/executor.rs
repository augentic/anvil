//! In-process composed executor for the wasm profiles.
//!
//! One executor hosts the deployment through `omnia` exactly the way
//! the shipped binary does — one fresh command-mode deployment per
//! workflow step over the same manifest — with the model backend
//! injected by constructor: [`ComposedExecutor::replay`] reads the
//! canonical request-key fixtures the scenario materialises at
//! [`REPLAY_FIXTURES`]; [`ComposedExecutor::live`] connects the cursor
//! backend and serves the `/mcp/<name>` reference routes (bind address
//! from `HTTP_ADDR`, omnia's default otherwise) for the agents it
//! spawns.
//!
//! Scenario fidelity: fixtures are materialised into the trial
//! workspace and `expected-outputs` are graded by the trial loop;
//! non-empty `setup.commands` are rejected until a scenario needs
//! them, and non-empty environment additions are rejected because
//! omnia guests inherit the host environment with no per-store
//! injection seam.

mod capture;

use std::fs;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use anyhow::{Context as _, Result, anyhow, bail, ensure};
use omnia::wasmtime_wasi::ResourceTable;
use omnia::{
    Backend as _, Backends, Deployment, DeploymentBuilder, HasHttp, Mode, Runtime, Server as _,
    StoreCtx, Wiring, run,
};
use omnia_wasi_http::{HttpDefault, WasiHttp, WasiHttpCtxView};
use omnia_wasi_model::{HasModel, ModelDefault, WasiModel, WasiModelCtx};
use scenario::grade::{Execution, StepResult};
use scenario::{ModelBackend, Profile, Runtime as ScenarioRuntime, Scenario};

use self::capture::Capture;
use crate::manifest::Manifest;

/// Workspace-relative directory the replay model backend reads its
/// request-key fixtures from; scenarios materialise it through their
/// `fixtures` list.
pub const REPLAY_FIXTURES: &str = ".quality/replay";

/// The replay fixture directory for the deployment currently driving.
///
/// `omnia::Backends::connect` takes no arguments, so the executor
/// parks the per-trial directory here before each step. One executor
/// drives at a time per process; concurrent replay executors are
/// unsupported.
static REPLAY_DIR: Mutex<Option<PathBuf>> = Mutex::new(None);

/// One profile execution: drive a scenario's workflow in a trial
/// workspace and return the captured evidence.
pub trait Executor {
    /// Execute every workflow step of `scenario` under `profile` in
    /// `workspace`, stopping at the first failing step.
    ///
    /// # Errors
    ///
    /// Returns setup errors only; step failures are data on the
    /// returned [`Execution`].
    fn execute(
        &self, scenario: &Scenario, profile: &Profile, workspace: &Path,
    ) -> impl Future<Output = Result<Execution>>;
}

/// Which model backend a composed deployment binds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Model {
    Replay,
    Live,
}

/// In-process composed-WASM executor over the workflow guest and a set
/// of adapter components.
#[derive(Debug, Clone)]
pub struct ComposedExecutor {
    workflow: PathBuf,
    adapters: Vec<(String, PathBuf)>,
    stage: Vec<(PathBuf, String)>,
    seed: Option<Vec<String>>,
    fixtures_root: Option<PathBuf>,
    model: Model,
}

impl ComposedExecutor {
    /// A replay-backed executor over the workflow guest at `workflow`.
    #[must_use]
    pub fn replay(workflow: impl Into<PathBuf>) -> Self {
        Self::new(workflow, Model::Replay)
    }

    /// A live cursor-backed executor over the workflow guest at
    /// `workflow`.
    #[must_use]
    pub fn live(workflow: impl Into<PathBuf>) -> Self {
        Self::new(workflow, Model::Live)
    }

    fn new(workflow: impl Into<PathBuf>, model: Model) -> Self {
        Self {
            workflow: workflow.into(),
            adapters: Vec::new(),
            stage: Vec::new(),
            seed: None,
            fixtures_root: None,
            model,
        }
    }

    /// Add an adapter guest by dispatch id (`source:<name>` or
    /// `target:<name>`). Chainable.
    #[must_use]
    pub fn adapter(mut self, id: &str, component: impl Into<PathBuf>) -> Self {
        self.adapters.push((id.to_owned(), component.into()));
        self
    }

    /// Copy `file` into the trial workspace root as `name` before the
    /// workflow starts. Chainable.
    #[must_use]
    pub fn stage(mut self, file: impl Into<PathBuf>, name: &str) -> Self {
        self.stage.push((file.into(), name.to_owned()));
        self
    }

    /// Run one clerical `init` leg (guest argv without the leading
    /// binary name) before the scenario workflow. Chainable.
    #[must_use]
    pub fn seed(mut self, argv: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.seed = Some(argv.into_iter().map(Into::into).collect());
        self
    }

    /// Directory scenario fixture `source` paths resolve against.
    /// Chainable; required when the scenario declares fixtures.
    #[must_use]
    pub fn fixtures_root(mut self, root: impl Into<PathBuf>) -> Self {
        self.fixtures_root = Some(root.into());
        self
    }
}

impl Executor for ComposedExecutor {
    async fn execute(
        &self, scenario: &Scenario, profile: &Profile, workspace: &Path,
    ) -> Result<Execution> {
        self.check(scenario, profile)?;
        let workspace = self.prepare(scenario, workspace)?;
        let manifest = workspace.join("omnia.toml");
        fs::write(&manifest, self.manifest(&workspace).render())
            .context("writing the deployment manifest")?;

        let mut plan: Vec<(String, Vec<String>)> = Vec::new();
        if let Some(seed) = &self.seed {
            plan.push(("init".to_owned(), seed.clone()));
        }
        for step in &scenario.workflow {
            let argv = step.argv().map_err(|error| anyhow!("{error}"))?;
            plan.push((step.id.clone(), argv[1..].to_vec()));
        }

        let steps = match self.model {
            Model::Replay => {
                *REPLAY_DIR.lock().expect("the replay-dir slot is never poisoned") =
                    Some(workspace.join(REPLAY_FIXTURES));
                drive::<ReplayBundle, Quiet>(&manifest, plan).await?
            }
            Model::Live => drive::<LiveBundle, Serving>(&manifest, plan).await?,
        };
        Ok(Execution::new(workspace, steps))
    }
}

impl ComposedExecutor {
    fn check(&self, scenario: &Scenario, profile: &Profile) -> Result<()> {
        ensure!(
            profile.runtime == ScenarioRuntime::Wasm,
            "profile `{}` is not a wasm profile; native profiles run through the adapters \
             harness",
            profile.id
        );
        let declared = match profile.model {
            ModelBackend::Replay => Model::Replay,
            ModelBackend::Live => Model::Live,
            ModelBackend::Scripted => bail!(
                "profile `{}` declares a scripted model, which the composed executor does not \
                 host",
                profile.id
            ),
        };
        ensure!(
            declared == self.model,
            "profile `{}` declares a {declared:?} model but the executor binds {:?}",
            profile.id,
            self.model
        );
        ensure!(
            scenario.setup.commands.is_empty(),
            "scenario `{}` declares setup commands, which no executor supports yet",
            scenario.id
        );
        ensure!(
            scenario.setup.environment.is_empty() && profile.environment.is_empty(),
            "scenario `{}` declares environment additions; omnia guests inherit the host \
             environment with no per-store injection seam, so per-trial environment is \
             unsupported",
            scenario.id
        );
        Ok(())
    }

    /// Create the workspace and cache mounts, stage executor files,
    /// and materialise scenario fixtures; returns the canonical root.
    fn prepare(&self, scenario: &Scenario, workspace: &Path) -> Result<PathBuf> {
        fs::create_dir_all(workspace.join(".specify-cache"))
            .with_context(|| format!("creating the workspace at {}", workspace.display()))?;
        let workspace = workspace.canonicalize().context("resolving the workspace root")?;
        for (file, name) in &self.stage {
            fs::copy(file, workspace.join(name))
                .with_context(|| format!("staging {}", file.display()))?;
        }
        for fixture in &scenario.fixtures {
            let root = self.fixtures_root.as_deref().with_context(|| {
                format!("scenario `{}` declares fixtures; set fixtures_root", scenario.id)
            })?;
            copy_tree(&root.join(&fixture.source), &workspace.join(&fixture.destination))
                .with_context(|| format!("materialising fixture `{}`", fixture.id))?;
        }
        Ok(workspace)
    }

    fn manifest(&self, workspace: &Path) -> Manifest {
        let mut manifest = Manifest::workflow(&self.workflow).mount(".", workspace, true).mount(
            "/specify-cache",
            &workspace.join(".specify-cache"),
            true,
        );
        for (id, component) in &self.adapters {
            manifest = manifest.guest(id, component);
            if self.model == Model::Live {
                manifest = manifest.mcp_route(id);
            }
        }
        manifest
    }
}

/// Drive each planned step through one fresh command-mode deployment,
/// capturing the process streams; stops at the first failing step.
async fn drive<B, H>(
    manifest: &Path, plan: Vec<(String, Vec<String>)>,
) -> Result<Vec<(String, StepResult)>>
where
    B: Backends,
    H: Wiring<B>,
{
    let mut steps = Vec::new();
    for (id, argv) in plan {
        eprintln!("==> specify {}", argv.join(" "));
        let capture = Capture::start().context("capturing the process streams")?;
        let status = run::<B, H>(
            DeploymentBuilder::new().config(manifest.to_path_buf()).args(argv).mode(Mode::Command),
        )
        .await;
        let (stdout, stderr) =
            capture.finish().context("restoring the captured process streams")?;
        let step = match status {
            Ok(status) => StepResult {
                exit_code: status.code(),
                stdout,
                stderr,
            },
            // Deployment errors are step evidence, not setup errors:
            // the failing step stays in the transcript for grading.
            Err(error) => StepResult {
                exit_code: -1,
                stdout,
                stderr: format!("{stderr}\ndeployment error: {error:#}"),
            },
        };
        eprint!("{}", step.stdout);
        eprint!("{}", step.stderr);
        let failed = step.exit_code != 0;
        steps.push((id, step));
        if failed {
            break;
        }
    }
    Ok(steps)
}

fn copy_tree(source: &Path, destination: &Path) -> Result<()> {
    if source.is_dir() {
        fs::create_dir_all(destination)
            .with_context(|| format!("creating {}", destination.display()))?;
        for entry in
            fs::read_dir(source).with_context(|| format!("reading {}", source.display()))?
        {
            let entry = entry?;
            copy_tree(&entry.path(), &destination.join(entry.file_name()))?;
        }
    } else {
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
        }
        fs::copy(source, destination).with_context(|| format!("copying {}", source.display()))?;
    }
    Ok(())
}

/// Replay-backed bundle: HTTP default plus the request-key fixture
/// store parked in [`REPLAY_DIR`] by the executor.
#[derive(Clone)]
struct ReplayBundle {
    http: HttpDefault,
    model: ModelDefault,
}

impl std::fmt::Debug for ReplayBundle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReplayBundle").finish_non_exhaustive()
    }
}

impl Backends for ReplayBundle {
    async fn connect() -> Result<Self> {
        let dir = REPLAY_DIR
            .lock()
            .expect("the replay-dir slot is never poisoned")
            .clone()
            .context("the composed replay executor parks the fixture dir before driving")?;
        Ok(Self {
            http: HttpDefault::connect().await?,
            model: ModelDefault::from_dir(dir)?,
        })
    }
}

/// Live bundle: HTTP default plus the cursor model backend — the same
/// pair the shipped binary's `omnia::runtime!` binds.
#[derive(Clone)]
struct LiveBundle {
    http: HttpDefault,
    model: omnia_cursor::Client,
}

impl std::fmt::Debug for LiveBundle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LiveBundle").finish_non_exhaustive()
    }
}

impl Backends for LiveBundle {
    async fn connect() -> Result<Self> {
        let (http, model) =
            tokio::try_join!(HttpDefault::connect(), omnia_cursor::Client::connect())?;
        Ok(Self { http, model })
    }
}

macro_rules! bundle_views {
    ($bundle:ty) => {
        impl HasHttp for $bundle {
            fn http_view<'a>(&'a mut self, table: &'a mut ResourceTable) -> WasiHttpCtxView<'a> {
                self.http.as_view(table)
            }
        }

        impl HasModel for $bundle {
            fn model_ctx(&mut self) -> &mut dyn WasiModelCtx {
                &mut self.model
            }
        }
    };
}

bundle_views!(ReplayBundle);
bundle_views!(LiveBundle);

/// Replay wiring: hosts linked, no trigger servers.
#[derive(Debug)]
struct Quiet;

impl Wiring<ReplayBundle> for Quiet {
    fn link(deployment: &mut Deployment<StoreCtx<ReplayBundle>>) -> Result<()> {
        deployment.host::<WasiHttp, ReplayBundle>()?;
        deployment.host::<WasiModel, ReplayBundle>()?;
        Ok(())
    }

    async fn serve(_runtime: &Runtime<ReplayBundle>) -> Result<()> {
        Ok(())
    }
}

/// Live wiring: hosts linked plus the HTTP trigger serving the
/// `/mcp/<name>` reference routes.
#[derive(Debug)]
struct Serving;

impl Wiring<LiveBundle> for Serving {
    fn link(deployment: &mut Deployment<StoreCtx<LiveBundle>>) -> Result<()> {
        deployment.host::<WasiHttp, LiveBundle>()?;
        deployment.host::<WasiModel, LiveBundle>()?;
        Ok(())
    }

    async fn serve(runtime: &Runtime<LiveBundle>) -> Result<()> {
        // Each step spawns a fresh deployment; the first step's server
        // holds the bind address for the whole trial (identical
        // manifest, stateless reference reads), so later bind failures
        // are expected and inert.
        WasiHttp.run(runtime).await
    }
}
