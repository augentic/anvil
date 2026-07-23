//! The shared live composition client (`feature = "client"`).
//!
//! One argv dispatch serving both composition examples — the `eval`
//! example in this repository over the mock catalog and the one in
//! `augentic/specify-adapters` over the first-party catalog: native
//! command passthrough by default, the live trial under the `eval`
//! subcommand. The client owns the lazily connected cursor backend
//! ([`DevModel`]) and the `--project-dir` convenience; the
//! composition root keeps what the client refuses to own — the Tokio
//! runtime, `std::env::args` collection, and the catalog and
//! prompt-scenario declarations.

mod model;
mod native;

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use ::native::{Catalog, DynModel, ExecutionPaths};
pub use model::DevModel;

use crate::{ModelFactory, ModelInstance};

/// Dispatch one composition-binary invocation over `catalog`.
///
/// The `eval` subcommand routes through [`crate::run`] (with
/// `scenarios` as the prompt-scenario root, when the composition
/// carries one); anything else runs through the native command API.
///
/// # Errors
///
/// Returns `--project-dir` resolution failures and every [`crate::run`]
/// failure.
pub async fn run(
    mut argv: Vec<String>, catalog: Catalog, scenarios: Option<&Path>,
) -> anyhow::Result<ExitCode> {
    // `cargo make specify -- ARGS` forwards the literal `--` separator.
    if argv.get(1).is_some_and(|arg| arg == "--") {
        argv.remove(1);
    }
    let root = project_root(&mut argv)?;

    if argv.get(1).is_some_and(|arg| arg == "eval") {
        return crate::run(root, catalog, cursor_factory(), &argv[1..], scenarios).await;
    }

    let paths = ExecutionPaths::operator(root.clone());
    let model = DynModel::new(DevModel::new(&root));
    Ok(::native::command::run(paths, model, catalog, argv).await)
}

/// A lazily connected cursor-agent backend per phase root, carrying
/// the `SPECIFY_EVAL_MODEL` default read once at composition.
#[must_use]
pub fn cursor_factory() -> ModelFactory {
    let default = std::env::var("SPECIFY_EVAL_MODEL").ok().filter(|id| !id.trim().is_empty());
    Arc::new(move |root| {
        Ok(ModelInstance {
            model: DynModel::new(DevModel::new(root)),
            default_model: default.clone(),
        })
    })
}

/// Resolve the lab's canonical anchor: the `--project-dir` option when
/// placed before the subcommand, else the current directory.
fn project_root(argv: &mut Vec<String>) -> anyhow::Result<PathBuf> {
    let dir = take_project_dir(argv).map_err(|message| anyhow::anyhow!(message))?;
    let dir = dir.unwrap_or_else(|| PathBuf::from("."));
    dir.canonicalize().map_err(|error| anyhow::anyhow!("--project-dir {}: {error}", dir.display()))
}

// Only the option before the subcommand is the lab's; later `--project-dir` passes through.
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
