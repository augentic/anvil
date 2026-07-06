//! Composed-deployment host surface for the `specify` binary.
//!
//! Hand-rolls the backend bundle and [`omnia::Wiring`] the `runtime!`
//! macro would generate for `hosts: { WasiHttp: HttpDefault, WasiOtel:
//! OtelDefault, WasiModel: Cursor }` — the macro emits its own private
//! `main`, so a binary that triages argv between native handlers and
//! the composed deployment supplies the equivalent by hand over
//! omnia's public surface and awaits [`omnia::run`] itself. The root
//! `specify` binary drives guest-owned verbs through [`drive`]; the
//! replay sibling (`specify-runtime-replay`, `src/bin/replay.rs`)
//! keeps the macro over `ModelDefault` for component-level tests and
//! examples. See `DECISIONS.md` §"One `specify` binary".

#![cfg(not(target_arch = "wasm32"))]

pub mod describe;

use std::path::Path;

use anyhow::Result;
use omnia::wasmtime_wasi::ResourceTable;
use omnia::{
    Backend as _, Backends, Deployment, DeploymentBuilder, HasHttp, Mode, Runtime, Server as _,
    StoreCtx, Wiring,
};
use omnia_cursor::Client as Cursor;
use omnia_wasi_http::{HttpDefault, WasiHttp, WasiHttpCtxView};
use omnia_wasi_model::{HasModel, WasiModel, WasiModelCtx};
use omnia_wasi_otel::{HasOtel, OtelDefault, WasiOtel, WasiOtelCtx};

/// The release-built workflow-guest component, embedded at compile time.
///
/// Sourced from the committed artifact at `crates/workflow-guest/guest.wasm`
/// (regenerate with `cargo make dist-guest` after changing guest-reachable
/// code). The triage dispatch in the `specify` binary stages these bytes
/// into its transient deployment manifest so a released binary is
/// self-contained. See `DECISIONS.md` §"Workflow-guest distribution".
pub const WORKFLOW_GUEST_WASM: &[u8] =
    include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../workflow-guest/guest.wasm"));

/// The operational backend bundle: real HTTP trigger, OpenTelemetry
/// defaults, and the spawning `cursor-agent` model backend.
#[derive(Debug, Clone)]
pub struct CursorBundle {
    http: HttpDefault,
    otel: OtelDefault,
    cursor: Cursor,
}

impl Backends for CursorBundle {
    async fn connect() -> Result<Self> {
        let (http, otel, cursor) =
            tokio::try_join!(HttpDefault::connect(), OtelDefault::connect(), Cursor::connect())?;
        Ok(Self { http, otel, cursor })
    }
}

impl HasHttp for CursorBundle {
    fn http_view<'a>(&'a mut self, table: &'a mut ResourceTable) -> WasiHttpCtxView<'a> {
        self.http.as_view(table)
    }
}

impl HasOtel for CursorBundle {
    fn otel_ctx(&mut self) -> &mut dyn WasiOtelCtx {
        &mut self.otel
    }
}

impl HasModel for CursorBundle {
    fn model_ctx(&mut self) -> &mut dyn WasiModelCtx {
        &mut self.cursor
    }
}

/// Host linking and trigger startup for the cursor-bound deployment:
/// `wasi:http` (the MCP shelves), `wasi:otel`, and `wasi:model` link;
/// the HTTP trigger is the only long-lived server.
#[derive(Debug, Clone, Copy)]
pub struct Hooks;

impl Wiring<CursorBundle> for Hooks {
    fn link(deployment: &mut Deployment<StoreCtx<CursorBundle>>) -> Result<()> {
        deployment.host::<WasiHttp, CursorBundle>()?;
        deployment.host::<WasiOtel, CursorBundle>()?;
        deployment.host::<WasiModel, CursorBundle>()?;
        Ok(())
    }

    async fn serve(runtime: &Runtime<CursorBundle>) -> Result<()> {
        WasiHttp.run(runtime).await
    }
}

/// Drive one guest CLI invocation through the composed deployment in
/// command mode.
///
/// Builds the deployment from `manifest`, forwards `args` as the guest
/// argv (the runtime core prepends the deployment name as `argv[0]`),
/// serves the HTTP trigger in the background, and returns the guest's
/// exit status (low byte, POSIX semantics) for process passthrough.
///
/// Blocks on its own multi-thread Tokio runtime so the synchronous
/// dispatch layer of the `specify` binary can call it directly.
///
/// # Errors
///
/// Returns an error when the Tokio runtime cannot be built, the
/// deployment cannot be assembled (manifest, guests, mounts), or a
/// backend fails to connect (`cursor-agent` missing from `PATH`).
pub fn drive(manifest: &Path, args: Vec<String>) -> Result<u8> {
    let builder =
        DeploymentBuilder::new().config(manifest.to_path_buf()).args(args).mode(Mode::Command);
    let runtime = tokio::runtime::Builder::new_multi_thread().enable_all().build()?;
    let status = runtime.block_on(omnia::run::<CursorBundle, Hooks>(builder))?;
    Ok(status.code_u8())
}
