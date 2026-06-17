//! Dispatcher for `specify archive *`. Owns the `archive prune`
//! retention GC over `.specify/archive/`.

use std::io::Write;

use serde::Serialize;
use specify_error::{Error, Result};
use specify_workflow::slice::actions::{Retention, prune};

pub mod cli;

use cli::ArchiveAction;

use crate::runtime::context::Ctx;

pub fn run(ctx: &Ctx, action: &ArchiveAction) -> Result<()> {
    match *action {
        ArchiveAction::Prune {
            keep,
            older_than,
            dry_run,
        } => prune_archive(ctx, keep, older_than, dry_run),
    }
}

fn prune_archive(
    ctx: &Ctx, keep: Option<usize>, older_than: Option<i64>, dry_run: bool,
) -> Result<()> {
    if keep.is_none() && older_than.is_none() {
        return Err(Error::Argument {
            flag: "--keep/--older-than",
            detail:
                "supply at least one retention bound (`--keep <n>` and/or `--older-than <days>`)"
                    .to_string(),
        });
    }
    let retention = Retention {
        keep,
        max_age_days: older_than,
    };
    let archive_dir = ctx.archive_dir();
    let candidates = prune::scan(&archive_dir, retention, ctx.now())?;
    if !dry_run {
        prune::prune(&candidates)?;
    }
    let pruned: Vec<String> = candidates.into_iter().map(|e| e.name).collect();

    ctx.write(
        &PruneBody {
            dry_run,
            pruned: &pruned,
        },
        write_prune_text,
    )
}

#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct PruneBody<'a> {
    dry_run: bool,
    pruned: &'a [String],
}

fn write_prune_text(w: &mut dyn Write, body: &PruneBody<'_>) -> std::io::Result<()> {
    let verb = if body.dry_run { "Would prune" } else { "Pruned" };
    if body.pruned.is_empty() {
        return writeln!(w, "Nothing to prune.");
    }
    writeln!(w, "{verb} {} archived slice(s):", body.pruned.len())?;
    for name in body.pruned {
        writeln!(w, "  {name}")?;
    }
    Ok(())
}
