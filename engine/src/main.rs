//! `specify` binary entry point. Thin shim over the runtime library
//! module; see `DECISIONS.md` for the exit-code contract.

use std::process::ExitCode;

fn main() -> ExitCode {
    specify::runtime::run()
}
