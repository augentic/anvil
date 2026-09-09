//! The `emery` executable
//!
//! The shipped runtime: one omnia deployment that embeds the engine guest and
//! declares everything it is allowed to touch. The invocation directory is
//! mounted read-only as the project, revision state is kept under
//! `.omnia/storage`, the model is Cursor, and adapters may be loaded from
//! local `.wasm` files or the `omnia.host` registry.
//!
//! The deployment is fixed at compile time so a given `emery` binary always
//! runs with the same policy; there is no runtime configuration to audit.

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
        }
    ],
    mounts: [
        { name: ".", path: "." },
    ],
    plugins: {
        interfaces: ["emery:adapter/source@0.1.0"],
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
