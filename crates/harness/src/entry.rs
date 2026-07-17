//! Shared native entrypoint: CLI dev shim by default, live trial under
//! `eval`.

use std::path::Path;
use std::process::ExitCode;

use crate::catalog::Binding;
use crate::{command, trial};

/// Run the wrapper binary as one call from its `main`.
///
/// `eval` runs the shared live trial; every other invocation goes
/// through the native CLI shim. `scenarios` is the binding-owned
/// scenario root, when that binding supports prompt scenarios.
#[must_use]
pub fn main<B: Binding>(scenarios: Option<&Path>) -> ExitCode {
    let runtime = match tokio::runtime::Builder::new_multi_thread().enable_all().build() {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("eval: building the tokio runtime: {error}");
            return ExitCode::FAILURE;
        }
    };
    let argv: Vec<String> = std::env::args().collect();
    runtime.block_on(run::<B>(argv, scenarios))
}

async fn run<B: Binding>(argv: Vec<String>, scenarios: Option<&Path>) -> ExitCode {
    let outcome = match argv.get(1).map(String::as_str) {
        Some("eval") => trial::run::<B>(&argv[1..], scenarios).await,
        _ => return ExitCode::from(command::run::<B>(argv).await),
    };
    outcome.unwrap_or_else(|err| {
        eprintln!("eval: {err:#}");
        ExitCode::FAILURE
    })
}
