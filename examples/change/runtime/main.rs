//! Greeting example runtime.
//!
//! `omnia.toml` deploys the Specify workflow guest with the fixture adapter
//! bound on both axes. The runtime replaces the live model with deterministic
//! answers from [`scripted`].

#[cfg(not(target_arch = "wasm32"))]
mod scripted;

cfg_if::cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
        use omnia_wasi_http::{HttpDefault, WasiHttp};
        use omnia_wasi_model::WasiModel;
        use omnia_wasi_otel::{OtelDefault, WasiOtel};
        use scripted::Scripted;

        omnia::runtime!({
            mode: command,
            hosts: {
                WasiHttp: HttpDefault,
                WasiOtel: OtelDefault,
                WasiModel: Scripted,
            }
        });
    } else {
        fn main() {}
    }
}
