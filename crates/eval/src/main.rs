//! `eval` — the shared harness wrapper binary over the fixture
//! catalog: CLI dev shim (default), HTTP (`serve`), and the
//! live-model trial (`eval`).

use std::process::ExitCode;

use eval::{Adapters, SHELL};

fn main() -> ExitCode {
    harness::entry::main::<Adapters>(&SHELL)
}
