//! Specify's unpublished composition binary: linked command
//! passthrough over the mock catalog by default, the live eval
//! client under `eval`.
//!
//! The composition root owns what `linked` and `eval` refuse to: the
//! Tokio runtime, `std::env::args`, the lab-only `--project-dir`
//! convenience, Cursor backend construction, and the catalog
//! declaration. It is a development tool, never an install or release
//! artifact.

mod model;
mod native;

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use eval::{ModelFactory, ModelInstance};
use linked::{DynModel, ExecutionPaths};

use crate::model::DevModel;

#[tokio::main]
async fn main() -> ExitCode {
    match entry().await {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}

async fn entry() -> anyhow::Result<ExitCode> {
    let mut argv: Vec<String> = std::env::args().collect();
    // `cargo make specify -- ARGS` forwards the literal `--` separator.
    if argv.get(1).is_some_and(|arg| arg == "--") {
        argv.remove(1);
    }
    let root = project_root(&mut argv)?;
    let catalog = mock::catalog();

    if argv.get(1).is_some_and(|arg| arg == "eval") {
        return eval::run(root, catalog, cursor_factory(), &argv[1..], None).await;
    }

    let paths = ExecutionPaths::operator(root.clone());
    let model = DynModel::new(DevModel::new(&root));
    Ok(linked::command::run(paths, model, catalog, argv).await)
}

/// A lazily connected cursor-agent backend per phase root, carrying
/// the `SPECIFY_EVAL_MODEL` default read once at composition.
fn cursor_factory() -> ModelFactory {
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
