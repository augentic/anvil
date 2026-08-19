//! The emery guest (wasm32) — the deployment's only `wasi:cli/run`
//! exporter — or, on native, the `launcher` deployment-policy module
//! the binary and the journey host share (ADR-0011).

#[cfg(target_arch = "wasm32")]
guest::export!();

#[cfg(not(target_arch = "wasm32"))]
pub mod launcher;
