//! `specify` — the generic Omnia command-mode runtime over the
//! cursor-bound backends. The binary carries no Specify vocabulary:
//! every verb runs in the specify (core) guest, driven through
//! omnia's own `run` grammar (`specify run <wasm|--config manifest>
//! -- <guest argv>`, `OMNIA_CONFIG` as the manifest fallback). The
//! replay sibling (`runtime-replay`) keeps the same macro over
//! `ModelDefault` for on-demand rig runs.

// cfg_if::cfg_if! {
//     if #[cfg(not(target_arch = "wasm32"))] {
        use omnia_cursor::Client as Cursor;
        use omnia_wasi_http::{HttpDefault, WasiHttp};
        use omnia_wasi_model::WasiModel;
        use omnia_wasi_otel::{OtelDefault, WasiOtel};

        omnia::runtime!({
            mode: command,
            hosts: {
                WasiHttp: HttpDefault,
                WasiOtel: OtelDefault,
                WasiModel: Cursor,
            }
        });
//     } else {
//         fn main() {}
//     }
// }
