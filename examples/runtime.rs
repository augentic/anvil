//! Journey runtime
//!
//! A development build of the shipped runtime for walking the live
//! `specify` → `show` journey against the mock adapter built by `make source`.
//! It mirrors the shipped deployment except that adapters load only from
//! local paths, so the journey never reaches out to a registry.
//!
//! Run it with `specify --config examples/emery.toml`.

cfg_if::cfg_if! {
    if #[cfg(target_arch = "wasm32")] {
        fn main() {}
    } else {
        use omnia_cursor::Client as Cursor;
        use omnia_filesystem::{Client as Filesystem, ConnectOptions};
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
                },
            ],
            mounts: [
                { name: ".", path: "." },
            ],
            link: {
                interfaces: ["emery:adapter/source@0.1.0"],
            },
            plugin: {
                locations: [
                    { name: ".", path: "." },
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
