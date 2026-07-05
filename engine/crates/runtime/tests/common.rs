//! Shared helpers for the composed-deployment integration tests.
//!
//! Owns guest-artifact building/locating (this workspace's counterpart to
//! `omnia_testkit::find_guest`, pointed at the `specify-*-guest` crates), the
//! `wasi:http`-backed store bundle the host binary's `runtime!` macro would
//! generate, and the walking-skeleton manifest the tests deploy.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use anyhow::Result;
use omnia::wasmtime_wasi::ResourceTable;
use omnia::{Backend as _, Deployment, HasHttp, Runtime, StoreCtx, Wiring};
use omnia_testkit::{TempManifest, temp_manifest};
use omnia_wasi_http::{HttpDefault, WasiHttp, WasiHttpCtxView};
use omnia_wasi_model::{HasModel, ModelDefault, WasiModel, WasiModelCtx};

/// Built artifact name of the echo source-adapter guest.
pub const ECHO_WASM: &str = "specify_echo_guest.wasm";

/// Built artifact name of the workflow (`wasi:cli/run`) guest.
pub const WORKFLOW_WASM: &str = "specify_workflow_guest.wasm";

/// The engine workspace root (`<root>/crates/runtime` is this crate).
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
        "guest `{file}` not found at {path}; run `cargo make build-guests` from engine/",
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
            .args([
                "build",
                "-p",
                "specify-echo-guest",
                "-p",
                "specify-workflow-guest",
                "--target",
                "wasm32-wasip2",
            ])
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

/// The skeleton deployment manifest (`engine/omnia.toml` with absolute guest
/// and mount paths), registering the echo guest under `echo_id`. The `"."`
/// mount is part of the skeleton's contract — the workflow guest hard-fails
/// when its preopen table lacks it.
///
/// # Errors
///
/// Returns an error when the temp manifest cannot be written.
pub fn skeleton_manifest(echo_id: &str) -> Result<TempManifest> {
    let workflow = guest_wasm(WORKFLOW_WASM);
    let echo = guest_wasm(ECHO_WASM);
    let mount = workspace_root().join("crates/runtime/workspace");

    temp_manifest(&format!(
        "[[guest]]\n\
         id = \"workflow\"\n\
         source.path = \"{workflow}\"\n\
         link = [\"augentic:specify/source@0.1.0\", \"augentic:specify/target@0.1.0\"]\n\n\
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

/// The backend bundle the host binary's `runtime!` macro generates for
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

/// Test wiring: link the `wasi:http` host (the echo guest imports its
/// types) and the `wasi-model` replay host (the workflow guest imports
/// `omnia:model/completion` for its judgment legs), but serve no trigger
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
