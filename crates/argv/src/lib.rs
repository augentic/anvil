//! Wasm-clean `specify` argv transport.
//!
//! Owns the typed command router, clap-to-operation-input conversions,
//! output envelopes, and exit-code contract.
//!
//! The operation bodies themselves live in `workflow`'s domain modules
//! (each family in a `handlers` submodule beside its kernels); the
//! WASI and native shims construct the same reusable route assembly.
//!
//! Wasm specifics stay out: this crate never depends on wit-bindgen,
//! wasip3, or wasmtime — the guest shim owns all WIT binding.

pub mod cli;
pub mod commands;
pub mod http;
pub mod output;
pub mod router;
