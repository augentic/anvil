//! `specify` library surface — clap parse, dispatch, and exit mapping.
//!
//! The argv grammar, output envelopes, exit-code contract, and pure
//! workflow verb handlers live in the wasm-clean `specify-dispatch`
//! crate (re-exported here); this module keeps the native-only
//! handlers. See `DECISIONS.md` for the exit-code contract.

pub(crate) mod commands;

use std::process::ExitCode;

use clap::Parser;
pub use specify_dispatch::output;
pub use specify_dispatch::output::Exit;
pub(crate) use specify_dispatch::{cli, context};

/// Parse argv once, triage the verb, and return the process exit code.
/// The `specify` binary calls into this.
///
/// Guest-owned verbs (the collapsed orchestrators the native handler
/// table refuses) route to the composed deployment through
/// `commands::guest`; everything else dispatches in-process through
/// today's handlers. See `DECISIONS.md` §"One `specify` binary".
#[must_use]
pub fn run() -> ExitCode {
    let cli = cli::Cli::parse();
    if commands::guest::owned(&cli.command) {
        return commands::guest::run(cli.format, cli.plan_dir.as_deref()).into();
    }
    commands::run(cli).into()
}
