//! The Omnia deployment unit: on `wasm32`, the engine guest exporting the
//! deployment's CLI entry point; natively, the shipped runtime declared once
//! through `omnia::runtime!` — the binary runs it (`main`) and the component
//! rung overlays it (`manifest`, `Hooks`).

cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        use emery_engine::cli;

        omnia_guest::provider! {
            struct Provider: Model + StateStore + BlobStore + Plugins;
        }
        impl emery_source::Source for Provider {}

        omnia_guest::command!(dispatch);

        async fn dispatch() -> Result<(), u8> {
            let response = cli::router(Provider)
                .execute(wasip3::cli::environment::get_arguments())
                .await;
            if response.write_to(&mut std::io::stdout(), &mut std::io::stderr()).is_err() {
                return Err(3);
            }
            if response.exit == 0 { Ok(()) } else { Err(response.exit) }
        }
    } else {
        use omnia_cursor::Client as Cursor;
        use omnia_filesystem::{Client as Filesystem, ConnectOptions};
        use omnia_wasi_blobstore::WasiBlobstore;
        use omnia_wasi_keyvalue::WasiKeyValue;
        use omnia_wasi_model::WasiModel;
        use omnia_wasi_otel::{OtelDefault, WasiOtel};

        // The static, CWD-rooted deployment policy: the invocation directory
        // mounts read-only as `.`, generation state binds to the durable
        // filesystem store, and the source seam loads local `.wasm` adapters
        // from the `.` path root or exact package references from the
        // `omnia.host` registry (read fresh; no project cache).
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
            plugins: {
                interfaces: [emery_source::SOURCE_INTERFACE],
                locations: [
                    { name: ".", path: "." },
                    { registry: "omnia.host" },
                ],
            },
            hosts: {
                WasiOtel: OtelDefault,
                WasiModel: Cursor,
                WasiKeyValue: Filesystem(ConnectOptions { root: ".omnia/storage".into() }),
                WasiBlobstore: Filesystem(ConnectOptions { root: ".omnia/storage".into() }),
            }
        });
    }
}
