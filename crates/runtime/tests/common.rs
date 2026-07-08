//! Shared helpers for the composed-deployment integration tests.
//!
//! Owns guest-artifact building/locating (this workspace's counterpart to
//! `omnia_testkit::find_guest`, pointed at the `{echo-source,echo-target,specify}` guest crates), the
//! hand-rolled backend bundles mirroring what the host binaries' `runtime!`
//! macros generate, and the skeleton manifest the tests deploy.
//!
//! **Test-only in-process harness.** The product path is the `specify`
//! binary — one `omnia::runtime!` invocation over the cursor-bound
//! backends; these bundles exist because omnia's telemetry `OnceLock`
//! allows only one `omnia::run` per process, so multi-assertion suites
//! drive the deployment in-process over stubbed backends instead of
//! through the product binary.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, OnceLock};

use anyhow::Result;
use omnia::futures::FutureExt as _;
use omnia::wasmtime_wasi::ResourceTable;
use omnia::{Backend as _, Deployment, HasHttp, Runtime, StoreCtx, Wiring};
use omnia_testkit::{TempManifest, temp_manifest};
use omnia_wasi_http::{HttpDefault, WasiHttp, WasiHttpCtxView};
use omnia_wasi_model::{
    Answer, FutureResult, HasModel, ModelDefault, Request, ToolHost, WasiModel, WasiModelCtx,
};

/// Built artifact name of the echo source-adapter guest.
pub const ECHO_WASM: &str = "echo_source.wasm";

/// Built artifact name of the workflow (`wasi:cli/run`) guest.
pub const SPECIFY_WASM: &str = "specify.wasm";

/// The repo workspace root (`<root>/crates/runtime` is this crate).
pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("runtime manifest dir is <workspace>/crates/runtime")
        .to_path_buf()
}

/// Locate a built wasm32-wasip2 guest component, building the guest crates on
/// first use (a fast no-op when fresh).
///
/// # Panics
///
/// Panics when the artifact is still absent after the build, pointing the
/// developer at `cargo make build-guests`.
pub fn guest_wasm(file: &str) -> PathBuf {
    build_guests();

    let path = target_dir().join("wasm32-wasip2").join("debug").join(file);
    assert!(
        path.exists(),
        "guest `{file}` not found at {path}; run `cargo make build-guests` from the repo root",
        path = path.display()
    );
    path
}

// Build the skeleton guest crates once per test process; cargo's own build
// lock serializes concurrent invocations across test binaries.
fn build_guests() {
    static GUESTS: OnceLock<()> = OnceLock::new();
    GUESTS.get_or_init(|| {
        let status = Command::new("cargo")
            .env("CARGO_TARGET_DIR", target_dir())
            .args(["build", "-p", "echo-source", "-p", "workflow", "--target", "wasm32-wasip2"])
            .current_dir(workspace_root())
            .status()
            .expect("spawning guest build");
        assert!(status.success(), "guest build failed with status {status}");
    });
}

// The cargo target dir this test binary was built into (testkit's convention:
// the test exe sits at `<target>/<profile>/deps/<exe>`).
fn target_dir() -> PathBuf {
    let test_exe = std::env::current_exe().expect("test executable has a path");
    test_exe
        .ancestors()
        .nth(3)
        .expect("test exe sits at <target>/<profile>/deps/<exe>")
        .to_path_buf()
}

/// The sibling `augentic/specify-adapters` checkout carrying the adapter
/// guest sources the composed deployment builds on (release-built via
/// `cargo make release` there; adapters ship as single-file components,
/// never as committed `guest.wasm` artifacts).
///
/// # Panics
///
/// Panics when the checkout is absent — the composed workflow tests hard-
/// require it (same posture as the `omnia` path pins in `Cargo.toml`).
pub fn adapters_root() -> PathBuf {
    let root = workspace_root()
        .parent()
        .expect("the specify repo root has a parent")
        .join("specify-adapters");
    assert!(
        root.is_dir(),
        "sibling augentic/specify-adapters checkout not found at {root}; the composed \
         deployment tests require it (see the repo-root omnia.toml)",
        root = root.display()
    );
    root
}

/// The sibling checkout's release-built component for one adapter,
/// addressed by its manifest guest id (`source:intent`, `target:omnia`,
/// …), mirroring the checked-in repo-root `omnia.toml` paths. Locate-
/// only: the one-time `cargo make release` in the sibling checkout is a
/// developer prerequisite (this repo's tests never drive a build in the
/// sibling workspace).
///
/// # Panics
///
/// Panics when the id has no axis prefix or the artifact is absent.
pub fn adapter_component_wasm(id: &str) -> PathBuf {
    let (_axis, name) = id.split_once(':').expect("adapter guest id is `<axis>:<name>`");
    let path = adapters_root()
        .join("target/wasm32-wasip2/release")
        .join(format!("{}.wasm", name.replace('-', "_")));
    assert!(
        path.exists(),
        "adapter component not found at {path}; run `cargo make release` in the sibling \
         specify-adapters checkout",
        path = path.display()
    );
    path
}

/// A composed-deployment manifest: the workflow guest plus the
/// given release-built adapter components (each with its `/mcp/<name>` route),
/// sharing one writable `"."` mount at `mount` — the shape of the
/// checked-in repo-root `omnia.toml` over a test-owned project tree —
/// plus the per-project derived cache mounted at the guest cache
/// preopen (guest routing), mirroring the generated manifest.
///
/// # Errors
///
/// Returns an error when the temp manifest cannot be written.
pub fn composed_manifest(mount: &Path, adapters: &[&str]) -> Result<TempManifest> {
    use std::fmt::Write as _;

    let workflow = guest_wasm(SPECIFY_WASM);
    let mut doc = format!(
        "[[guest]]\n\
         id = \"workflow\"\n\
         source.path = \"{workflow}\"\n\
         link = [\"specify:adapter/source@0.1.0\", \"specify:adapter/target@0.1.0\"]\n\n",
        workflow = workflow.display(),
    );
    for id in adapters {
        let wasm = adapter_component_wasm(id);
        writeln!(doc, "[[guest]]\nid = \"{id}\"\nsource.path = \"{}\"\n", wasm.display())?;
    }
    writeln!(doc, "[[mount]]\nname = \".\"\npath = \"{}\"\nwritable = true\n", mount.display())?;
    // The mount registry opens every mount at deployment build, so the
    // cache dir must exist even when a test never touches it.
    let cache = schema::cache::project_cache_dir(mount);
    std::fs::create_dir_all(&cache)?;
    writeln!(
        doc,
        "[[mount]]\nname = \"{name}\"\npath = \"{path}\"\nwritable = true\n",
        name = schema::cache::GUEST_CACHE_MOUNT,
        path = cache.display(),
    )?;
    for id in adapters {
        let name = id.split_once(':').expect("guest id is `<axis>:<name>`").1;
        writeln!(doc, "[[route.http]]\nprefix = \"/mcp/{name}\"\nguest = \"{id}\"\n")?;
    }
    doc.push_str("[transport]\ndefault = \"in-process\"\n");
    temp_manifest(&doc)
}

/// Assemble a composed deployment (workflow + release-built adapters, `"."`
/// mounted at `mount`) into a runtime the in-process HTTP driver can
/// serve requests through, over the stubbed model backend.
///
/// # Errors
///
/// Returns an error when the deployment cannot be built or the backends
/// cannot connect.
pub async fn composed_runtime(mount: &Path, adapters: &[&str]) -> Result<Runtime<StubBundle>> {
    use anyhow::Context as _;
    use omnia::Backends as _;

    let manifest = composed_manifest(mount, adapters)?;
    let mut deployment = omnia::DeploymentBuilder::new()
        .config(manifest.path().to_path_buf())
        .build::<StoreCtx<StubBundle>>()
        .await
        .context("building deployment")?;
    deployment.host::<WasiHttp, StubBundle>().context("linking http host")?;
    deployment.host::<WasiModel, StubBundle>().context("linking model host")?;
    let mounts = deployment.mounts();
    let registry = deployment.into_registry().context("assembling registry")?;

    Ok(Runtime::from_parts(
        Arc::new(registry),
        Vec::new(),
        mounts,
        StubBundle::connect().await.context("connecting backends")?,
    ))
}

/// The skeleton deployment manifest (the repo-root `omnia.toml` with absolute
/// guest and mount paths), registering the echo guest under `echo_id`. The `"."`
/// mount is part of the skeleton's contract — the workflow guest hard-fails
/// when its preopen table lacks it.
///
/// # Errors
///
/// Returns an error when the temp manifest cannot be written.
pub fn skeleton_manifest(echo_id: &str) -> Result<TempManifest> {
    let workflow = guest_wasm(SPECIFY_WASM);
    let echo = guest_wasm(ECHO_WASM);
    let mount = workspace_root().join("crates/runtime/workspace");

    temp_manifest(&format!(
        "[[guest]]\n\
         id = \"workflow\"\n\
         source.path = \"{workflow}\"\n\
         link = [\"specify:adapter/source@0.1.0\", \"specify:adapter/target@0.1.0\"]\n\n\
         [[guest]]\n\
         id = \"{echo_id}\"\n\
         source.path = \"{echo}\"\n\n\
         [[mount]]\n\
         name = \".\"\n\
         path = \"{mount}\"\n\
         writable = true\n\n\
         [[route.http]]\n\
         prefix = \"/mcp/echo\"\n\
         guest = \"{echo_id}\"\n\n\
         [transport]\n\
         default = \"in-process\"\n",
        workflow = workflow.display(),
        echo = echo.display(),
        mount = mount.display(),
    ))
}

const CACHE_ENV: &str = "SPECIFY_PROJECT_CACHE";

/// Restores the previous `SPECIFY_PROJECT_CACHE` value on drop.
pub struct CacheGuard(Option<std::ffi::OsString>);

impl Drop for CacheGuard {
    #[expect(unsafe_code, reason = "restore the cache-root env var pinned for the test")]
    fn drop(&mut self) {
        // SAFETY: nextest runs each test in its own process, so no other
        // thread observes the env mutation for the guard's lifetime.
        unsafe {
            match self.0.take() {
                Some(prev) => std::env::set_var(CACHE_ENV, prev),
                None => std::env::remove_var(CACHE_ENV),
            }
        }
    }
}

/// Pin the out-of-tree project cache root inside `dir` so cache writes
/// (native seeding and the guest's cache mount alike) are hermetic and
/// auto-cleaned with the tempdir.
#[expect(unsafe_code, reason = "pin the cache-root env var into the test tempdir")]
pub fn scoped_cache(dir: &Path) -> CacheGuard {
    let prev = std::env::var_os(CACHE_ENV);
    // SAFETY: see `CacheGuard::drop` — single-process test isolation.
    unsafe { std::env::set_var(CACHE_ENV, dir.join("project-cache")) };
    CacheGuard(prev)
}

/// The backend bundle the `runtime!` macro generates for
/// `hosts: { WasiHttp: HttpDefault, WasiModel: ModelDefault }`.
#[derive(Clone)]
pub struct Bundle {
    http: HttpDefault,
    model: ModelDefault,
}

impl omnia::Backends for Bundle {
    async fn connect() -> Result<Self> {
        Ok(Self {
            http: HttpDefault::connect().await?,
            model: ModelDefault::connect().await?,
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

/// The model-free bundle for the composed workflow tests: `wasi:http`
/// real, the model backend stubbed with [`NoModel`] — the tested flows
/// are model-free by design (judgment legs are covered natively against
/// `MockModel`), so any completion reaching the host is a test bug.
#[derive(Clone)]
pub struct StubBundle {
    http: HttpDefault,
    model: NoModel,
}

impl omnia::Backends for StubBundle {
    async fn connect() -> Result<Self> {
        Ok(Self {
            http: HttpDefault::connect().await?,
            model: NoModel,
        })
    }
}

impl HasHttp for StubBundle {
    fn http_view<'a>(&'a mut self, table: &'a mut ResourceTable) -> WasiHttpCtxView<'a> {
        self.http.as_view(table)
    }
}

impl HasModel for StubBundle {
    fn model_ctx(&mut self) -> &mut dyn WasiModelCtx {
        &mut self.model
    }
}

/// A model backend that fails every completion: linked so the guests'
/// `omnia:model/completion` import resolves, never legitimately reached.
#[derive(Clone, Debug)]
struct NoModel;

impl WasiModelCtx for NoModel {
    fn complete(&self, _request: Request, _tool_host: Arc<dyn ToolHost>) -> FutureResult<Answer> {
        async {
            // Yield through the reactor before failing so the guest's
            // async-lifted export genuinely parks awaiting the import — the
            // probe must prove the seam survives a pending host future, not
            // just an immediately-ready one.
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            anyhow::bail!("model-free composed test: completion must not be called")
        }
        .boxed()
    }
}

/// Test wiring: link the `wasi:http` host (the adapter guests import its
/// types) and the `wasi-model` host (the guests import
/// `omnia:model/completion` for their judgment legs), but serve no trigger
/// servers — the tests never bind a TCP socket.
pub struct Quiet;

impl Wiring<Bundle> for Quiet {
    fn link(deployment: &mut Deployment<StoreCtx<Bundle>>) -> Result<()> {
        deployment.host::<WasiHttp, Bundle>()?;
        deployment.host::<WasiModel, Bundle>()?;
        Ok(())
    }

    async fn serve(_runtime: &Runtime<Bundle>) -> Result<()> {
        Ok(())
    }
}

impl Wiring<StubBundle> for Quiet {
    fn link(deployment: &mut Deployment<StoreCtx<StubBundle>>) -> Result<()> {
        deployment.host::<WasiHttp, StubBundle>()?;
        deployment.host::<WasiModel, StubBundle>()?;
        Ok(())
    }

    async fn serve(_runtime: &Runtime<StubBundle>) -> Result<()> {
        Ok(())
    }
}
