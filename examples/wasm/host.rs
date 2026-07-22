//! Macro-static example host: the same `omnia::runtime!` shape as the
//! shipped binary, demonstrating all three guest admission routes in
//! one deployment — the engine embedded as static component bytes
//! (`include_bytes!`), the mock target registered as a path-static
//! guest (static-wins: its dispatch never consults the resolver), and
//! the mock source faulting in dynamically through the launcher's
//! fail-closed resolver from the project component cache.
//!
//! Run it with `cargo make wasm-static-run` (operator-invoked, live
//! model). wasm32 builds compile an empty stub so `--examples` passes.

#[cfg(not(target_arch = "wasm32"))]
use omnia_cursor::Client as Cursor;
#[cfg(not(target_arch = "wasm32"))]
use omnia_wasi_http::{HttpDefault, WasiHttp};
#[cfg(not(target_arch = "wasm32"))]
use omnia_wasi_model::WasiModel;
#[cfg(not(target_arch = "wasm32"))]
use omnia_wasi_otel::{OtelDefault, WasiOtel};

#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(not(target_arch = "wasm32"))]
omnia::runtime!({
    mode: command,
    program: "specify-example",
    guests: [
        {
            id: "specify",
            source: include_bytes!(env!("SPECIFY_ENGINE_WASM")),
        },
        {
            id: "target:mock",
            source: concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/target/wasm32-wasip2/debug/examples/mock.wasm",
            ),
        },
    ],
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
