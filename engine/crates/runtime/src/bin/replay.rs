//! `specify-runtime-replay` — the replay sibling of the Specify runtime
//! binary (RFC-61 §target shape).
//!
//! Binds `WasiModel: ModelDefault` (recorded answers from
//! `MODEL_REPLAY_DIR`) over the same deployment manifest the `specify`
//! binary's guest leg drives (`specify_runtime::drive`), so
//! component-level tests and examples run without `cursor-agent` on
//! `PATH`. Not an operational mode.

cfg_if::cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
        use omnia_wasi_http::{HttpDefault, WasiHttp};
        use omnia_wasi_model::{ModelDefault, WasiModel};
        use omnia_wasi_otel::{OtelDefault, WasiOtel};

        omnia::runtime!({
            mode: command,
            hosts: {
                WasiHttp: HttpDefault,
                WasiOtel: OtelDefault,
                WasiModel: ModelDefault,
            }
        });
    } else {
        fn main() {}
    }
}
