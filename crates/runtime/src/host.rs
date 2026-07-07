//! The generic Omnia host layer (RFC-65 move 2), mounted in-process.
//!
//! The macro-generated command-mode runtime over the operational
//! backends: real HTTP trigger, OpenTelemetry defaults, and the
//! spawning `cursor-agent` model backend. Carries no Specify
//! vocabulary — it only drives the deployment manifest it is given and
//! forwards argv to the deployment's `wasi:cli/run` guest. The
//! `specify` binary's guest leg blocks on [`drive`] through the
//! crate-level [`drive`](crate::drive) seam; the replay sibling
//! (`specify-runtime-replay`, `src/bin/replay.rs`) keeps the same
//! macro over `ModelDefault` for component-level tests and examples.
//! No wasm32 guard is needed here: the crate root carries
//! `#![cfg(not(target_arch = "wasm32"))]`.

use omnia_cursor::Client as Cursor;
use omnia_wasi_http::{HttpDefault, WasiHttp};
use omnia_wasi_model::WasiModel;
use omnia_wasi_otel::{OtelDefault, WasiOtel};

omnia::runtime!({
    mode: command,
    hosts: {
        WasiHttp: HttpDefault,
        WasiOtel: OtelDefault,
        WasiModel: Cursor,
    }
});
