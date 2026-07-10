//! Wasm-clean `specify` CLI front-end.
//!
//! Owns the full clap argv grammar ([`cli::Cli`] / [`cli::Commands`]),
//! the clap-to-`Input` conversions the dispatch matches use to feed
//! the transport-neutral command handlers in `workflow`, the output
//! envelopes ([`output::Format`] / [`output::emit`]), the exit-code
//! contract ([`output::Exit`] / [`output::report`]), and the
//! [`front::run`] bridge that drives one command `Handler` and renders
//! its `Reply` (or failure) onto stdout/stderr.
//!
//! The handler bodies themselves live in `workflow`'s domain modules
//! (each family in a `handlers` submodule beside its kernels); the
//! exhaustive per-shim dispatch matches live in the shims (the wasm
//! guest in `src/lib.rs`, the native `specify-dev` binary) —
//! deliberately duplicated so the compiler checks each shim's
//! coverage of [`cli::Commands`].
//!
//! Wasm specifics stay out: this crate never depends on wit-bindgen,
//! wasip3, or wasmtime — the guest shim owns all WIT binding.

pub mod cli;
pub mod commands;
pub mod front;
pub mod output;

/// Alias over [`front::run`] documenting write intent on a routing
/// arm — the argv counterpart of `route::post` on the HTTP side.
pub use front::run as post;
/// Alias over [`front::run`] documenting read intent on a routing
/// arm — the argv counterpart of `route::get` on the HTTP side.
pub use front::run as get;
