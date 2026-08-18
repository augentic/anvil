//! Asynchronous native command execution over the shared typed Emery
//! router. Runtime-free — the composition root owns Tokio.

use std::io;
use std::process::ExitCode;

use omnia_guest::api::command::CommandResponse;
use omnia_guest::api::invoke::Invoker;
use project::handler::ExecutionPaths;

use crate::catalog::Catalog;
use crate::error::Error;
use crate::model::DynModel;
use crate::provider::Provider;

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
    let provider = Provider::new(paths, model, catalog);
    let router = transport::command::router(Invoker::new("emery", provider)).map_err(|err| {
        Error::Router {
            detail: err.to_string(),
        }
    })?;
    Ok(transport::command::execute(&router, argv).await)
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
