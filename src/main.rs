//! The shipped `emery` executable: one `omnia::runtime!` invocation.

// Create the CWD-relative cache
#[cfg(not(target_arch = "wasm32"))]

cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        fn main() {}
    } else {
        use omnia_cursor::Client as Cursor;
        use omnia_wasi_http::{HttpDefault, WasiHttp};
        use omnia_wasi_model::WasiModel;
        use omnia_wasi_otel::{OtelDefault, WasiOtel};

        omnia::runtime!({
            mode: command,
            program: "emery",
            command_guest: "emery",
            guests: [
                {
                    id: "emery",
                    source: include_bytes!(concat!(env!("OUT_DIR"), "/emery.cwasm")),
                },
            ],
            mounts: [
                { name: ".", path: ".", writable: true },
                { name: emery_engine::handler::GUEST_CACHE_MOUNT, path: cache_dir(), writable: true },
            ],
            hosts: {
                WasiHttp: HttpDefault,
                WasiOtel: OtelDefault,
                WasiModel: Cursor,
            }
        });

        fn cache_dir() -> &'static str {
            drop(std::fs::create_dir_all(".emery-cache"));
            ".emery-cache"
        }
    }
}
