//! # Change Runtime
//!
//! This is the runtime for the change example. It can use either the Cursor
//! (live, probablistic) or Scripted (deterministic) models to generate answers.

// #[cfg(not(target_arch = "wasm32"))]
// mod scripted;

cfg_if::cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
        use omnia_wasi_http::{HttpDefault, WasiHttp};
        use omnia_wasi_model::WasiModel;
        use omnia_wasi_otel::{OtelDefault, WasiOtel};
        // use scripted::Scripted;
        use omnia_cursor::Client as Cursor;

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
