//! Asynchronous native command execution over the shared typed Emery
//! router. Runtime-free — the composition root owns Tokio; [`execute`]
//! awaits reference-listener shutdown on every exit path.

use std::io;
use std::process::ExitCode;

use omnia_guest::api::command::CommandResponse;
use omnia_guest::api::invoke::Invoker;
use project::config::Roots;
use project::handler::ExecutionPaths;
use transport::command::selectors::change_request;

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
    let paths = reanchor(paths, &argv);
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

/// Apply `--change-dir` when argv names it; otherwise keep the
/// caller's paths (native tests pass a tempdir while cwd is the crate).
fn reanchor(paths: ExecutionPaths, argv: &[String]) -> ExecutionPaths {
    let rest: Vec<String> = argv.iter().skip(1).cloned().collect();
    let Some(dir) = change_request(&rest).change_dir else {
        return paths;
    };
    let roots = Roots::resolve(paths.project_root(), Some(&dir));
    ExecutionPaths::from_roots(&roots, paths.locations().clone())
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
