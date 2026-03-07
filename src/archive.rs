use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::engine::Engine;
use crate::{pipeline, registry, status};

/// Archive a completed change. In v3 this is hub-side only:
/// verify all target PRs are merged, then move the change folder to archive.
pub fn run(change: &str, dry_run: bool, engine: &dyn Engine, workspace: &Path) -> Result<()> {
    let changes_dir = workspace.join(engine.changes_dir());
    let change_dir = changes_dir.join(change);
    let pipeline = pipeline::Pipeline::load(&change_dir.join("pipeline.toml"))?;
    let reg = registry::Registry::load(&workspace.join("registry.toml"))?;
    pipeline.validate(&reg, &change_dir)?;

    let status_path = change_dir.join("status.toml");
    let pstatus =
        status::PipelineStatus::load_or_create(&status_path, change, &pipeline, &reg)?;

    let not_merged: Vec<_> = pstatus
        .targets
        .iter()
        .filter(|t| !t.state.is_at_least(status::TargetState::Merged))
        .collect();

    if !not_merged.is_empty() {
        let names: Vec<_> = not_merged.iter().map(|t| format!("{} ({})", t.id, t.state)).collect();
        bail!("cannot archive: targets not yet merged: {}", names.join(", "));
    }

    let archive_dest =
        workspace.join(engine.archive_dir()).join(engine.archive_dirname(change));

    if dry_run {
        println!("=== DRY RUN: archive '{change}' ===\n");
        println!("  from: {}", change_dir.display());
        println!("    to: {}", archive_dest.display());
        println!("\nall targets merged. no changes made (dry run)");
        return Ok(());
    }

    if archive_dest.exists() {
        bail!("archive destination already exists: {}", archive_dest.display());
    }

    std::fs::create_dir_all(archive_dest.parent().unwrap())?;
    std::fs::rename(&change_dir, &archive_dest).with_context(|| {
        format!("moving {} to {}", change_dir.display(), archive_dest.display())
    })?;

    println!("archived to {}", archive_dest.display());
    Ok(())
}
