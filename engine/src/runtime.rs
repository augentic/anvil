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

/// Parse argv, dispatch the subcommand, and return the process exit
/// code. The `specify` binary calls into this.
#[must_use]
pub fn run() -> ExitCode {
    let cli = cli::Cli::parse();
    commands::run(cli).into()
}
