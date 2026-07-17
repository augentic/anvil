//! Native command entry over the shared typed Specify router.

use std::io;
use std::path::PathBuf;

use anyhow::{Result, ensure};
use omnia_guest::Model;
use omnia_guest::api::Provider as ApiProvider;
use omnia_guest::api::invoke::Invoker;
use project::adapter::{Hydrator, Resolver};
use project::handler::Anchor;
use project::seam::{Source, Target};

use crate::catalog::Binding;
use crate::model::DevModel;
use crate::provider::Provider;

/// Run one verb through the shared typed command router against
/// `provider`, streaming its output and failing on a non-zero exit.
///
/// # Errors
///
/// Returns a router-assembly failure and any non-zero verb exit.
pub async fn invoke<P>(provider: &P, argv: &[impl AsRef<str> + Sync]) -> Result<()>
where
    P: ApiProvider + Anchor + Model + Resolver + Hydrator + Source + Target + Clone,
{
    let display = argv.iter().map(AsRef::as_ref).collect::<Vec<_>>().join(" ");
    eprintln!("==> specify {display}");
    let router = transport::command::router(Invoker::new("specify", provider.clone()))
        .map_err(|error| anyhow::anyhow!("building the command router: {error}"))?;
    let mut full: Vec<String> = vec!["specify".to_string()];
    full.extend(argv.iter().map(|arg| arg.as_ref().to_string()));
    let response = router.execute(full).await;
    drop(response.write_to(&mut io::stdout().lock(), &mut io::stderr().lock()));
    ensure!(response.exit == 0, "`specify {display}` exited {}", response.exit);
    Ok(())
}

/// Parse and execute one native command invocation over `B`'s linked
/// adapters: the dev-shim entry behind a wrapper binary's default
/// (non-`eval`) mode.
pub async fn run<B: Binding>(mut argv: Vec<String>) -> u8 {
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
    let model = DevModel::new(&root);
    let provider = Provider::bound::<B>(root, model).await;
    let router = match transport::command::router(Invoker::new("specify", provider)) {
        Ok(router) => router,
        Err(error) => {
            eprintln!("error: {error}");
            return 1;
        }
    };
    let response = router.execute(argv).await;
    if response.write_to(&mut io::stdout().lock(), &mut io::stderr().lock()).is_err() {
        return 1;
    }
    response.exit
}

// Only the option before the subcommand is the shim's; later `--project-dir` passes through.
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
