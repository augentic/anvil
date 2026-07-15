//! Omnia runtime

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
