//! The shipped `emery` executable: one `omnia::runtime!` invocation
//! (RFC-70 Stage 3).
//!
//! The engine guest is embedded as static component bytes (`build.rs`
//! resolves `EMERY_WASM`) and routed as the sole static
//! `wasi:cli/run` exporter; every adapter guest is faulted in mid-run
//! by exact routed id through the fail-closed launcher resolver,
//! which installs a missing package pin from the first-party OCI
//! registry (pull-on-miss) and verify-and-loads everything else.
//! The launcher's mount expressions anchor the
//! project root from argv and the working directory, and grant the
//! `adapter add` component directory as a read-only self-named
//! preopen. Every invocation runs in the guest — help, version,
//! grammar rejections, and `adapter add` included; argv and the
//! engine guest's exit code pass through byte-for-byte. There is no
//! `omnia.toml` and no `run --config` surface: the deployment exists
//! only in memory, per invocation.

cfg_if::cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
        use omnia_cursor::Client as Cursor;
        use omnia_wasi_http::{HttpDefault, WasiHttp};
        use omnia_wasi_model::WasiModel;
        use omnia_wasi_otel::{OtelDefault, WasiOtel};

        omnia::runtime!({
            mode: command,
            program: "emery",
            guests: [{
                id: "emery",
                source: include_bytes!(env!("EMERY_WASM")),
            }],
            mounts: [
                { name: ".", path: launcher::project_root(), writable: true },
                { name: launcher::CACHE_MOUNT, path: launcher::cache_dir(), writable: true },
                { name: launcher::seed_mount_name(), path: launcher::seed_mount_path() },
            ],
            link: ["emery:adapter/source@0.1.0", "emery:adapter/target@0.1.0"],
            resolver: launcher::resolver(),
            // Required MCP projection: `/mcp/<axis>/<name>[@<version>]`
            // reaches the adapter guest's own `wasi:http` handler (the
            // references shelf granted on every judgment leg). With a
            // fallback installed HTTP routing is table-driven only — the
            // engine guest never catches adapter MCP traffic, and an
            // unservable path is a warn + 404.
            http_fallback: launcher::mcp_fallback,
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
