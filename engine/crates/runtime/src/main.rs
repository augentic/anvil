//! Omnia `runtime!` host binary for the Specify deployment (RFC-61 Step 4,
//! Milestone F).
//!
//! Command mode drives the workflow guest's `wasi:cli/run` export once and
//! exits with its status; the HTTP trigger serves each adapter guest's MCP
//! route in the background for the spawned `cursor-agent`. Runs via
//! `cargo run -p specify-runtime -- run --config omnia.toml` from the
//! workspace root. Requires `cursor-agent` on `PATH` (checked at connect),
//! authenticated via `CURSOR_API_KEY` or a prior `cursor-agent login`; the
//! replay sibling (`specify-runtime-replay`) binds `ModelDefault` over the
//! same manifest surface for component-level tests and examples.

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
