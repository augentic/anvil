//! Native CLI and live eval over the linked adapters.

use std::process::ExitCode;

use eval_binding::Adapters;

fn main() -> ExitCode {
    harness::entry::main::<Adapters>()
}
