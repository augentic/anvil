use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::context::ChangeContext;
use crate::engine::Engine;
use crate::{output, status};

/// Archive a completed change: verify all target PRs are merged, then move
/// the change folder to the archive directory.
pub fn run(change: &str, dry_run: bool, engine: &dyn Engine, workspace: &Path) -> Result<()> {
    let ctx = ChangeContext::load(workspace, engine, change)?;

    let not_merged: Vec<_> = ctx
        .status
        .targets
        .iter()
        .filter(|t| !t.state.is_at_least(status::TargetState::Merged))
        .collect();

    if !not_merged.is_empty() {
        let names: Vec<_> = not_merged
            .iter()
            .map(|t| format!("{} ({})", t.id, t.state))
            .collect();
        bail!("cannot archive: targets not yet merged: {}", names.join(", "));
    }

    let archive_dest = workspace
        .join(engine.archive_dir())
        .join(engine.archive_dirname(change));

    if dry_run {
        output::dry_run_banner("archive", change);
        println!("  from: {}", ctx.change_dir.display());
        println!("    to: {}", archive_dest.display());
        println!("\nall targets merged.");
        output::dry_run_footer();
        return Ok(());
    }

    if archive_dest.exists() {
        bail!(
            "archive destination already exists: {}",
            archive_dest.display()
        );
    }

    if let Some(parent) = archive_dest.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating archive parent dir {}", parent.display()))?;
    }
    std::fs::rename(&ctx.change_dir, &archive_dest).with_context(|| {
        format!(
            "moving {} to {}",
            ctx.change_dir.display(),
            archive_dest.display()
        )
    })?;

    println!("archived to {}", archive_dest.display());
    Ok(())
}
