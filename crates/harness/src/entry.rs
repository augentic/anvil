//! The shared wrapper-binary entrypoint: one `main` body over the
//! trilevel mode surface (CLI dev shim, `serve`, `eval`), so wrapper
//! binaries differ only in their linked catalog and trial declaration.

use std::process::ExitCode;

use anyhow::Result;

use crate::catalog::Binding;
use crate::trial::Profile;
use crate::{command, http, trial};

/// One wrapper binary's declaration. The linked adapters arrive
/// through the [`Binding`] type parameter on [`main`]; everything
/// else a wrapper owns is here.
#[derive(Clone, Copy, Debug)]
pub struct Shell {
    /// The binary name, prefixed onto failure reports.
    pub name: &'static str,
    /// The wrapper's trial declaration, built on entering `eval` mode.
    pub profile: fn() -> Result<Profile>,
}

/// Run the wrapper binary as one call from its `main`.
///
/// `serve` is the native HTTP transport, `eval` the live-model trial
/// over the shell's profile, and any other argv the CLI dev shim.
/// Owns the tokio runtime and the failure report.
#[must_use]
pub fn main<B: Binding>(shell: &Shell) -> ExitCode {
    let runtime = match tokio::runtime::Builder::new_multi_thread().enable_all().build() {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("{}: building the tokio runtime: {error}", shell.name);
            return ExitCode::FAILURE;
        }
    };
    let argv: Vec<String> = std::env::args().collect();
    runtime.block_on(run::<B>(shell, argv))
}

async fn run<B: Binding>(shell: &Shell, argv: Vec<String>) -> ExitCode {
    let outcome = match argv.get(1).map(String::as_str) {
        Some("serve") => http::serve::<B>(&argv[1..]).await,
        Some("eval") => eval::<B>(shell, &argv[1..]).await,
        _ => return ExitCode::from(command::run::<B>(argv).await),
    };
    outcome.unwrap_or_else(|err| {
        eprintln!("{}: {err:#}", shell.name);
        ExitCode::FAILURE
    })
}

async fn eval<B: Binding>(shell: &Shell, argv: &[String]) -> Result<ExitCode> {
    trial::run::<B>(&(shell.profile)()?, argv).await
}
