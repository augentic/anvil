//! Asynchronous native command execution over the shared typed
//! Emery router.
//!
//! Value-consuming and runtime-free: the composition root owns Tokio,
//! `std::env::args`, execution-path construction, model backend
//! construction, and catalog construction. [`execute`] builds an
//! online-references provider, runs the shared command router, and
//! awaits reference-listener shutdown on every exit path before
//! returning the typed transport response.

use std::io;
use std::process::ExitCode;

use omnia_guest::api::command::CommandResponse;
use omnia_guest::api::invoke::Invoker;
use project::handler::ExecutionPaths;

use crate::catalog::Catalog;
use crate::error::Error;
use crate::model::DynModel;
use crate::provider::{Provider, ReferenceMode};

/// Run one command invocation (`argv[0]` is the binary name) and
/// return the buffered transport response.
///
/// # Errors
///
/// [`Error::Router`] when the typed route inventory fails to
/// assemble — a deterministic build defect, not an input failure.
pub async fn execute(
    paths: ExecutionPaths, model: DynModel, catalog: Catalog, argv: Vec<String>,
) -> Result<CommandResponse, Error> {
    let provider = Provider::new(paths, model, catalog, ReferenceMode::Online);
    let router =
        transport::command::router(Invoker::new("emery", provider.clone())).map_err(|err| {
            Error::Router {
                detail: err.to_string(),
            }
        })?;
    let response = transport::command::execute(&router, argv).await;
    provider.shutdown().await;
    Ok(response)
}

/// [`execute`], then write the response to this process's standard
/// streams and fold every failure into an exit code.
pub async fn run(
    paths: ExecutionPaths, model: DynModel, catalog: Catalog, argv: Vec<String>,
) -> ExitCode {
    match execute(paths, model, catalog, argv).await {
        Ok(response) => response
            .write_to(&mut io::stdout().lock(), &mut io::stderr().lock())
            .unwrap_or(ExitCode::FAILURE),
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}
