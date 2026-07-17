//! Shared native entrypoint: CLI dev shim by default, live trial under
//! `eval`.

use std::process::ExitCode;

use crate::catalog::Binding;
use crate::{command, trial};

/// Run the wrapper binary as one call from its `main`.
///
/// `eval` runs the shared live trial; every other invocation goes
/// through the native CLI shim.
#[must_use]
pub fn main<B: Binding>() -> ExitCode {
    let runtime = match tokio::runtime::Builder::new_multi_thread().enable_all().build() {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("eval: building the tokio runtime: {error}");
            return ExitCode::FAILURE;
        }
    };
    let argv: Vec<String> = std::env::args().collect();
    runtime.block_on(run::<B>(argv))
}

async fn run<B: Binding>(argv: Vec<String>) -> ExitCode {
    let outcome = match argv.get(1).map(String::as_str) {
        Some("eval") => trial::run::<B>(&argv[1..]).await,
        _ => return ExitCode::from(command::run::<B>(argv).await),
    };
    outcome.unwrap_or_else(|err| {
        eprintln!("eval: {err:#}");
        ExitCode::FAILURE
    })
}
