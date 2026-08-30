//! The shipped `emery` executable: one `omnia::runtime!` invocation.

cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        fn main() {}
    } else {
        use omnia::MountAcquire;
        use omnia_cursor::Client as Cursor;
        use omnia_filesystem::Client as Filesystem;
        use omnia_wasi_blobstore::WasiBlobstore;
        use omnia_wasi_keyvalue::WasiKeyValue;
        use omnia_wasi_model::WasiModel;
        use omnia_wasi_otel::{OtelDefault, WasiOtel};
        use omnia_wasm_pkg::{AcquireExt as _, RegistryAcquire};

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
            // packages fetch from `omnia.host` over the project CAS.
            plugins: {
                interfaces: ["emery:adapter/source@0.1.0"],
                acquire: MountAcquire
                    .or(RegistryAcquire::new("omnia.host").cached_at(".omnia/cache/wasm-pkg")),
            },
            hosts: {
                WasiOtel: OtelDefault,
                WasiModel: Cursor,
                WasiKeyValue: Filesystem,
                WasiBlobstore: Filesystem,
            }
        });
    }
}
