//! Native command entry over the shared typed Specify router.

use std::path::PathBuf;

use omnia_guest::api::invoke::Invoker;
use specify_dev::mcp;
use specify_dev::model::DevModel;
use specify_dev::provider::Provider;
use tokio::net::TcpListener;

/// Split a leading shim-global `--project-dir <path>` /
/// `--project-dir=<path>` off `argv` — the CLI-mode counterpart of
/// serve mode's flag, so any verb can run against another project
/// without changing directory. Only the option *before* the subcommand
/// is the shim's; a per-verb `--project-dir` further right passes
/// through to the shared router untouched.
fn take_project_dir(argv: &mut Vec<String>) -> Result<Option<PathBuf>, String> {
    let Some(first) = argv.get(1).cloned() else {
        return Ok(None);
    };
    if first == "--project-dir" {
        let Some(path) = argv.get(2).cloned() else {
            return Err("--project-dir requires a path".to_string());
        };
        argv.drain(1..=2);
        return Ok(Some(PathBuf::from(path)));
    }
    if let Some(path) = first.strip_prefix("--project-dir=") {
        let path = PathBuf::from(path);
        argv.remove(1);
        return Ok(Some(path));
    }
    Ok(None)
}

/// Parse and execute one native command invocation.
pub async fn run(mut argv: Vec<String>) -> u8 {
    let root = match take_project_dir(&mut argv) {
        Ok(Some(dir)) => match dir.canonicalize() {
            Ok(root) => root,
            Err(error) => {
                eprintln!("error: --project-dir {}: {error}", dir.display());
                return 1;
            }
        },
        Ok(None) => PathBuf::from("."),
        Err(message) => {
            eprintln!("error: {message}");
            return 1;
        }
    };
    let model = match DevModel::from_env(&root) {
        Ok(model) => model,
        Err(error) => {
            eprintln!("error: {error:#}");
            return 1;
        }
    };
    let mut provider = Provider::new(root, model);
    if let Some(base) = shelves().await {
        provider = provider.mcp_base(base);
    }
    let router = match transport::command::router(Invoker::new("specify", provider), |_| Ok(())) {
        Ok(router) => router,
        Err(error) => {
            eprintln!("error: {error}");
            return 1;
        }
    };
    let response = router.execute(argv).await;
    if response.write_to(&mut std::io::stdout().lock(), &mut std::io::stderr().lock()).is_err() {
        return 1;
    }
    response.exit
}

async fn shelves() -> Option<String> {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.ok()?;
    let base = format!("http://127.0.0.1:{}", listener.local_addr().ok()?.port());
    tokio::spawn(async move {
        drop(axum::serve(listener, mcp::router()).await);
    });
    Some(base)
}
