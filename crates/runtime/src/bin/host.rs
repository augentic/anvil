//! `specify-host` — the generic Omnia host layer (RFC-65 move 2).
//!
//! The macro-generated command-mode runtime over the operational
//! backends: real HTTP trigger, OpenTelemetry defaults, and the
//! spawning `cursor-agent` model backend. Carries no Specify
//! vocabulary — it only reads the deployment manifest it is given
//! (`specify-host run --config <manifest> -- <argv>`) and forwards
//! argv to the deployment's `wasi:cli/run` guest, exit codes passing
//! through verbatim. The `specify` binary's guest leg spawns it
//! (`specify_runtime::drive`); the bare form stays available for
//! debugging and Omnia-native deployments.

cfg_if::cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
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
    } else {
        fn main() {}
    }
}
