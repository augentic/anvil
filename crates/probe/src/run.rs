//! The eval argv shim: parse `eval <case> [--until …] [--restart]`
//! and delegate to [`crate::case::run`].

use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result};
use clap::Parser;
use native::Catalog;

use crate::case::{self, ModelFactory, WorkflowUntil};

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
/// `workspace_root` anchors relative `cases` / `sandbox` paths. Both
/// roots are composition-owned — the shim does not derive one from the
/// other. Eval does not consult process current-directory state after
/// entry.
///
/// # Errors
///
/// Returns argument failures, a composition missing `cases` or
/// `sandbox`, and every [`case::run`] failure.
pub async fn run(
    workspace_root: PathBuf, catalog: Catalog, model: ModelFactory, args: &[String],
    cases: Option<&Path>, sandbox: Option<&Path>,
) -> Result<()> {
    let args = Args::parse_from(args);
    let cases = cases.context("this eval composition has no cases")?;
    let sandbox = sandbox.context("this eval composition has no sandbox")?;
    let cases = anchored(&workspace_root, cases);
    let sandbox = anchored(&workspace_root, sandbox);
    Box::pin(case::run(
        &cases,
        &sandbox,
        args.case.as_deref(),
        args.until,
        args.restart,
        &catalog,
        &model,
    ))
    .await
}

/// The case id argv names, when it parses as an eval invocation
/// naming one (a bare `eval` lists cases and names none).
#[cfg(feature = "client")]
pub fn case_of(args: &[String]) -> Option<String> {
    Args::try_parse_from(args).ok().and_then(|args| args.case)
}

pub fn anchored(workspace_root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() { path.to_path_buf() } else { workspace_root.join(path) }
}
