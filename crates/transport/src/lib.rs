//! Wasm-clean command and HTTP transport for `emery`.
//!
//! Owns the typed route assemblies, clap-to-operation-input
//! conversions, projectors, and exit-code contract.
//!
//! The operation bodies themselves live in `workflow`'s domain modules
//! (each family in a `handlers` submodule beside its kernels); the
//! WASI and native shims construct the same reusable route assemblies.
//!
//! Wasm specifics stay out: this crate never depends on wit-bindgen,
//! wasip3, or wasmtime — the guest shim owns all WIT binding.

pub mod command;
pub mod http;
