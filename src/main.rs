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
            guests: [{
                id: "emery",
                // AOT-serialized in release, raw wasm in debug
                // (adapters always stay raw and JIT at admission).
                source: include_bytes!(concat!(env!("OUT_DIR"), "/emery.bin")),
            }],
            mounts: [
                { name: ".", path: launcher::project_root(), writable: true },
                { name: launcher::CACHE_MOUNT, path: launcher::cache_dir(), writable: true },
            ],
            link: ["emery:adapter/source@0.1.0"],
            resolver: launcher::resolver(),
            // `/mcp/<axis>/<name>` reaches the adapter guest's own
            // `wasi:http` handler. Declined path or definitive miss →
            // 404; a fault on a claimed shelf → error-logged 500.
            http_paths: launcher::mcp_route,
            // Its local address becomes the guest-visible `HTTP_ADDR`
            // the adapter SDK derives grant URLs from.
            http_listener: launcher::http_listener(),
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
