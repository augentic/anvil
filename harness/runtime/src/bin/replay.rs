//! `runtime-replay` — the replay sibling of the `specify`
//! binary.
//!
//! The same `omnia::runtime!` command-mode macro, binding `WasiModel:
//! ModelDefault` (recorded answers from `MODEL_REPLAY_DIR`) instead of
//! the spawning cursor backend, so component-level tests and CI run
//! without `cursor-agent` on `PATH`. Not an operational mode.

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
