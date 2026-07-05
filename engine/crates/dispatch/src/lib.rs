//! Wasm-clean `specify` CLI dispatch (RFC-61 Step 4, Milestone D).
//!
//! Owns the full clap argv grammar ([`cli::Cli`] / [`cli::Commands`]),
//! the output envelopes ([`output::Format`] / [`output::emit`]), the
//! exit-code contract ([`output::Exit`] / [`output::report`]), the
//! project context ([`context::Ctx`]), and the handlers for every pure
//! workflow verb (`plan`, `slice`, `source`, `target`, `journal`).
//!
//! Two consumers share it:
//!
//! - the native `specify` binary routes its shared verbs through the
//!   [`commands`] entry points and keeps the native-only handlers
//!   (init, extension, lint, workspace, `plan lock`, `slice build`, …)
//!   in its own crate;
//! - the workflow guest shim (`crates/workflow-guest`) parses argv
//!   through [`guest::route`], runs the pure verbs in-process, and
//!   drives the [`guest::Orchestration`] verbs against its
//!   WIT-provided seam.
//!
//! Wasm specifics stay out: this crate never depends on wit-bindgen,
//! wasip3, wasmtime, or `specify-registry` — the guest shim owns all
//! WIT binding.

pub mod cli;
pub mod commands;
pub mod context;
pub mod guest;
pub mod output;
