//! Wasm-clean `specify` CLI dispatch.
//!
//! Owns the full clap argv grammar ([`cli::Cli`] / [`cli::Commands`]),
//! the output envelopes ([`output::Format`] / [`output::emit`]), the
//! exit-code contract ([`output::Exit`] / [`output::report`]), the
//! project context ([`context::Ctx`]), and the handlers for every pure
//! workflow verb (`plan`, `slice`, `source`, `target`, `journal`).
//!
//! One consumer: the workflow guest shim (`crates/workflow-guest`)
//! parses argv through [`guest::route`], runs the pure verbs
//! in-process, and drives the [`guest::Orchestration`] verbs against
//! its WIT-provided seam. The `specify` binary itself is a generic
//! Omnia runtime and never links this crate.
//!
//! Wasm specifics stay out: this crate never depends on wit-bindgen,
//! wasip3, or wasmtime — the guest shim owns all WIT binding.

pub mod cli;
pub mod commands;
pub mod context;
pub mod guest;
pub mod output;
