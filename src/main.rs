//! The shipped `emery` executable: one `omnia::runtime!` invocation.
//!
//! The engine guest is embedded as static component bytes from
//! `$OUT_DIR/emery.bin` — in release builds `build.rs` ahead-of-time
//! compiles it to the serialized wasmtime artifact, so startup
//! deserializes the engine instead of JIT-compiling it; debug builds
//! embed the raw component and JIT at startup — and routed as the
//! sole static
//! `wasi:cli/run` exporter; every adapter guest is faulted in mid-run
//! by exact routed id through the fail-closed launcher resolver,
//! which installs a missing package pin from the first-party OCI
//! registry (pull-on-miss) and verify-and-loads everything else.
//! The launcher's mount expressions anchor the
//! project root from argv and the working directory, and grant the
//! `adapter add` component directory as a read-only self-named
//! preopen. Every invocation runs in the guest — help, version,
//! grammar rejections, and `adapter add` included; argv and the
//! engine guest's exit code pass through byte-for-byte, except the
//! reserved host log flags (`--debug` / `--quiet`), which Omnia peels
//! into the host log preset before the guest sees argv. There is no
//! `omnia.toml` and no `run --config` surface: the deployment exists
//! only in memory, per invocation.
//!
//! The `WasiModel` slot takes `model::ModelBackend`, which connects
//! one spawned agent CLI — `cursor-agent` by default, Claude Code
//! under `EMERY_MODEL_BACKEND=claude` — and retries transport
//! failures with bounded backoff. Only the selected backend is
//! connected, so each mode requires only its own binary on `PATH`.

cfg_if::cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
        use model::ModelBackend;
        use omnia_wasi_http::{HttpDefault, WasiHttp};
        use omnia_wasi_model::WasiModel;
        use omnia_wasi_otel::{OtelDefault, WasiOtel};
        use omnia_wasi_workspaces::WasiWorkspaces;

        omnia::runtime!({
            mode: command,
            program: "emery",
            guests: [{
                id: "emery",
                // Build-time engine artifact: AOT-serialized in
                // release, raw wasm in debug (adapters always stay
                // raw wasm and JIT-compile at admission).
                source: include_bytes!(concat!(env!("OUT_DIR"), "/emery.bin")),
            }],
            mounts: [
                { name: ".", path: launcher::project_root(), writable: true },
                { name: launcher::CACHE_MOUNT, path: launcher::cache_dir(), writable: true },
                // Shared workspaces preopen: adapter guests open their
                // prepared private workspace by deployment-local path.
                // The snapshot store itself is host-owned — no mount.
                { name: launcher::WORKSPACES_MOUNT, path: launcher::workspaces_dir(), writable: true },
                { name: launcher::seed_mount_name(), path: launcher::seed_mount_path() },
            ],
            link: ["emery:adapter/source@0.1.0", "emery:adapter/target@0.1.0"],
            resolver: launcher::resolver(),
            // Required MCP route: `/mcp/<axis>/<name>[@<version>]` reaches
            // the adapter guest's own `wasi:http` handler (the references
            // shelf granted on every judgment leg). With the hook installed
            // HTTP routing is table-driven only — the engine guest never
            // catches adapter MCP traffic; a declined path or a definitive
            // resolver miss is an ordinary 404, while a genuine fault on a
            // claimed shelf (resolution failure, missing handler export)
            // is an error-logged 500.
            http_paths: launcher::mcp_route,
            // Pre-bound per-invocation listener (split bind policy); its
            // local address becomes the guest-visible `HTTP_ADDR` the
            // adapter SDK derives grant URLs from.
            http_listener: launcher::http_listener(),
            hosts: {
                WasiHttp: HttpDefault,
                WasiOtel: OtelDefault,
                WasiModel: ModelBackend,
                // The RFC-87 private-workspace capability: the engine
                // guest's `workspaces` import over the launcher's
                // snapshot-store kernel.
                WasiWorkspaces: launcher::Workspaces,
            }
        });
    } else {
        fn main() {}
    }
}
