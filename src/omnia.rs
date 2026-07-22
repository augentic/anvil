//! The shipped `specify` executable: one `omnia::runtime!` invocation
//! (RFC-70 Stage 3).
//!
//! The engine guest is embedded as static component bytes (`build.rs`
//! resolves `SPECIFY_ENGINE_WASM`) and routed as the sole static
//! `wasi:cli/run` exporter; every adapter guest is faulted in mid-run
//! by exact routed id through the fail-closed launcher resolver,
//! after the engine's own ensure legs have hydrated it through the
//! writable mounts. The launcher's mount expressions anchor the
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
            program: "specify",
            guests: [{
                id: "specify",
                source: include_bytes!(env!("SPECIFY_ENGINE_WASM")),
            }],
            mounts: [
                { name: ".", path: launcher::project_root(), writable: true },
                { name: launcher::CACHE_MOUNT, path: launcher::cache_dir(), writable: true },
                { name: launcher::STORE_MOUNT, path: launcher::store_dir(), writable: true },
                { name: launcher::seed_mount_name(), path: launcher::seed_mount_path() },
            ],
            link: ["specify:adapter/source@0.1.0", "specify:adapter/target@0.1.0"],
            resolver: launcher::resolver(),
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
