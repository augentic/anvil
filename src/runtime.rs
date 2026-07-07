//! `specify` library surface — the provisioning front of the shipped
//! binary.
//!
//! The binary triages on the first non-global-flag token only: a
//! token in [`cli::NATIVE_VERBS`] parses through the narrow native
//! provisioning grammar ([`cli::Cli`]) and dispatches in-process;
//! everything else — workflow verbs, `--help`, `--version`, bare
//! invocations — forwards unparsed to the workflow guest through the
//! composed deployment (`commands::guest::forward`). No workflow argv
//! is ever parsed natively, and no workflow verb is served natively.
//! The full operational grammar, output envelopes, and exit-code
//! contract live in the wasm-clean `specify-dispatch` crate
//! (re-exported here) and run in-guest. See `DECISIONS.md` for the
//! exit-code contract.

pub mod cli;
pub(crate) mod commands;

use std::process::ExitCode;

use clap::Parser;
pub(crate) use specify_dispatch::context;
pub use specify_dispatch::output;
pub use specify_dispatch::output::Exit;

/// Triage argv on the first token and return the process exit code.
/// The `specify` binary calls into this.
///
/// Forwarded verbs inherit stdio and pass exit codes through verbatim
/// (including clap's usage-error `2` from the in-guest parse); native
/// verbs keep today's envelopes. `--help` / `--version` / no-args
/// forward like everything else — the guest grammar retains the
/// provisioning verbs' definitions, so operator help stays whole while
/// provisioning execution stays native-only.
#[must_use]
pub fn run() -> ExitCode {
    commands::describe::register();
    if native(std::env::args().skip(1)) {
        let cli = cli::Cli::parse();
        return commands::run(cli).into();
    }
    commands::guest::forward().into()
}

/// Whether argv (minus the program name) leads with a native verb.
///
/// The only tokens skipped ahead of the verb are the two global flags
/// the native grammar accepts before the subcommand (`--format`,
/// `--plan-dir`, space- or `=`-separated), so `specify --format json
/// adapters sync` triages native exactly like `specify adapters sync
/// --format json`. Anything else in the lead position — another flag,
/// no token at all — forwards to the guest, whose clap tree owns the
/// rendering.
fn native(mut args: impl Iterator<Item = String>) -> bool {
    while let Some(token) = args.next() {
        match token.as_str() {
            "--format" | "--plan-dir" => {
                args.next();
            }
            other if other.starts_with("--format=") || other.starts_with("--plan-dir=") => {}
            other => return cli::NATIVE_VERBS.contains(&other),
        }
    }
    false
}
