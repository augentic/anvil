//! The shipped `emery` executable: one `omnia::runtime!` invocation.
//! Every invocation runs in the guest; argv and the exit code pass
//! through byte-for-byte except the peeled `--debug` / `--quiet`.

cfg_if::cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
        use emery::launcher;
        use omnia_cursor::Client as Cursor;
        use omnia_wasi_http::{HttpDefault, WasiHttp};
        use omnia_wasi_model::WasiModel;
        use omnia_wasi_otel::{OtelDefault, WasiOtel};

        omnia::runtime!({
            mode: command,
            program: "emery",
            guests: [
                {
                id: "emery",
                source: include_bytes!(concat!(env!("OUT_DIR"), "/emery.cwasm")),
                },
                {
                    id: "source:source",
                    source: include_bytes!(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/target/wasm32-wasip2/release/examples/source.wasm",
                    )),
                    link: ["emery:adapter/source@0.1.0"],
                },
            ],
            mounts: [
                { name: ".", path: launcher::project_root(), writable: true },
                { name: launcher::CACHE_MOUNT, path: launcher::cache_dir(), writable: true },
            ],
            // link: ["emery:adapter/source@0.1.0"],
            // resolver: launcher::resolver(),
            // http_paths: launcher::mcp_route,
            // http_listener: launcher::http_listener(),
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
