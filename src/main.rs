//! The shipped `emery` executable: one `omnia::runtime!` invocation.
//! Every invocation runs in the guest; argv and the exit code pass
//! through byte-for-byte except the peeled `--debug` / `--quiet`.

cfg_if::cfg_if! {
    if #[cfg(not(target_arch = "wasm32"))] {
        use omnia_cursor::Client as Cursor;
        use omnia_wasi_http::{HttpDefault, WasiHttp};
        use omnia_wasi_model::WasiModel;
        use omnia_wasi_blobstore::WasiBlobstore;
        use omnia_wasi_execmode::WasiExecMode;
        use omnia_wasi_otel::{OtelDefault, WasiOtel};
        use omnia_wasi_vcs::WasiVcs;

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
                // Shared workspaces preopen: adapter guests open their
                // prepared private workspace by deployment-local path.
                // The snapshot store itself is host-owned — no mount.
                { name: launcher::WORKSPACES_MOUNT, path: launcher::workspaces_dir(), writable: true },
                // VCS staging preopen: the host stages fetched trees
                // here; the engine guest reads and snapshots them.
                { name: launcher::STAGING_MOUNT, path: launcher::staging_dir() },
                { name: launcher::seed_mount_name(), path: launcher::seed_mount_path() },
                { name: launcher::definition_mount_name(), path: launcher::definition_mount_path() },
            ],
            link: ["emery:adapter/source@0.1.0", "emery:adapter/target@0.1.0"],
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
                // Exec-mode round-tripping for the in-guest workspace
                // kernel — `wasi:filesystem` carries no mode bits.
                WasiExecMode: launcher::ExecMode,
                // The snapshot object store: the filesystem blobstore
                // anchored at the launcher's snapshots root.
                WasiBlobstore: launcher::Blobstore,
                // Git and forge I/O for bind, archaeology, and
                // publication — the guest has no network or git;
                // trees land under the staging root.
                WasiVcs: launcher::Vcs,
            }
        });
    } else {
        fn main() {}
    }
}
