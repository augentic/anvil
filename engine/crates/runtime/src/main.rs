//! Omnia `runtime!` host binary for the Specify deployment (RFC-61 Step 1).
//!
//! Command mode drives the workflow guest's `wasi:cli/run` export once and
//! exits with its status; the HTTP trigger serves each adapter guest's MCP
//! route in the background. Runs via `cargo run -p specify-runtime -- run
//! --config omnia.toml` from the workspace root.

cfg_if::cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
        use omnia_wasi_http::{HttpDefault, WasiHttp};
        use omnia_wasi_model::{ModelDefault, WasiModel};

        // `WasiModel: ModelDefault` binds the replay backend so the
        // workflow guest's `omnia:model/completion` import links; the
        // operational cursor backend (RFC-61 §Step 2) swaps in when the
        // deployment goes live.
        omnia::runtime!({
            mode: command,
            hosts: {
                WasiHttp: HttpDefault,
                WasiModel: ModelDefault,
            }
        });
    } else {
        fn main() {}
    }
}
