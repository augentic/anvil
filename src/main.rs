//! The shipped `emery` executable: one `omnia::runtime!` invocation.

cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        fn main() {}
    } else {
        use omnia_cursor::Client as Cursor;
        use omnia_filesystem::{Client as Filesystem, ConnectOptions as FilesystemOptions};
        use omnia_wasi_blobstore::WasiBlobstore;
        use omnia_wasi_keyvalue::WasiKeyValue;
        use omnia_wasi_model::WasiModel;
        use omnia_wasi_otel::{OtelDefault, WasiOtel};

        omnia::runtime!({
            mode: command,
            guests: [
                {
                    id: "emery",
                    source: include_bytes!(concat!(env!("OUT_DIR"), "/emery.cwasm")),
                }
            ],
            mounts: [
                { name: ".", path: "." },
            ],
            // Paths resolve first (read fresh, never cached); registry
            // packages fetch from `omnia.host`, cached in the durable store.
            plugins: {
                interfaces: ["emery:adapter/source@0.1.0"],
                locations: [
                    { name: ".", path: "." },
                    { registry: "omnia.host" },
                ],
                cache: Filesystem,
            },
            hosts: {
                WasiOtel: OtelDefault,
                WasiModel: Cursor,
                // The generation store root is deployment policy, compiled
                // in — never `FILESYSTEM_ROOT`-tunable at run time.
                WasiKeyValue: Filesystem(FilesystemOptions { root: ".omnia/storage".into() }),
                WasiBlobstore: Filesystem(FilesystemOptions { root: ".omnia/storage".into() }),
            }
        });
    }
}
