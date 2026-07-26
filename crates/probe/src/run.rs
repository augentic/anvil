//! The eval argv shim: parse `eval <case> [--until …] [--restart]`
//! and delegate to [`crate::case::run`].

use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result};
use clap::Parser;
use native::Catalog;

use crate::case::{self, ModelFactory, WorkflowUntil};

/// The retained per-case sandbox root, a sibling of the composition
/// root's `cases/` directory.
const SANDBOX: &str = "sandbox";

#[derive(Debug, Parser)]
#[command(name = "eval", about = "Run one live eval case over native adapters")]
struct Args {
    /// Case id under the composition root's `cases/` directory; omit
    /// to list every case.
    case: Option<String>,
    /// Stop rung override; valid only for workflow cases.
    #[arg(long, value_enum)]
    until: Option<WorkflowUntil>,
    /// Replace the case's retained sandbox before running.
    #[arg(long)]
    restart: bool,
}

/// Run one eval case (or list them all when no case id is given).
///
/// `workspace_root` anchors a relative `cases` root; the retained
/// `sandbox/` tree lives beside the cases directory. Eval does not
/// consult process current-directory state after entry.
///
/// # Errors
///
/// Returns argument failures, a composition without cases, and every
/// [`case::run`] failure.
pub async fn run(
    workspace_root: PathBuf, catalog: Catalog, model: ModelFactory, args: &[String],
    cases: Option<&Path>,
) -> Result<()> {
    let args = Args::parse_from(args);
    let cases = cases.context("this eval composition has no cases")?;
    let cases = anchored(&workspace_root, cases);
    let sandbox = cases.parent().unwrap_or(&workspace_root).join(SANDBOX);
    case::run(&cases, &sandbox, args.case.as_deref(), args.until, args.restart, &catalog, &model)
        .await
}

fn anchored(workspace_root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() { path.to_path_buf() } else { workspace_root.join(path) }
}
