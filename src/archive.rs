use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::spec_engine::SpecEngine;
use crate::{agent, git, pipeline, registry, status};

pub fn run(change: &str, engine: &dyn SpecEngine, workspace: &Path) -> Result<()> {
    let change_dir = workspace.join("openspec/changes").join(change);
    let pipeline = pipeline::Pipeline::load(&change_dir.join("pipeline.toml"))?;
    let reg = registry::Registry::load(&workspace.join("registry.toml"))?;
    pipeline.validate(&reg, &change_dir)?;

    let status_path = change_dir.join("status.toml");
    let mut pstatus =
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

    let groups = pipeline.group_by_repo(&reg)?;

    for group in &groups {
        let all_archived = group.targets.iter().all(|t| {
            pstatus.get(&t.id).map(|s| s.state == status::TargetState::Archived).unwrap_or(false)
        });
        if all_archived {
            tracing::info!(repo = %group.repo, "already archived, skipping");
            continue;
        }

        tracing::info!(repo = %group.repo, "archiving");

        let svc = reg.find_by_id(&group.targets[0].id).context("target not in registry")?;

        let tmp = std::env::temp_dir().join(format!(
            "opsx-archive-{}-{}",
            group.targets[0].id,
            std::process::id()
        ));
        if tmp.exists() {
            std::fs::remove_dir_all(&tmp)?;
        }

        git::clone_shallow(&svc.repo, &tmp)?;

        let archive_cmd = engine.agent_archive_command(change);
        let succeeded = agent::invoke(&archive_cmd, &tmp)?;

        if succeeded {
            let msg = format!("opsx: archive {change}");
            let _ = git::add_commit_push(&tmp, &msg, "main");

            for t in &group.targets {
                pstatus.transition(&t.id, status::TargetState::Archived)?;
            }
            pstatus.save(&status_path)?;
            tracing::info!(repo = %group.repo, "archived");
        } else {
            tracing::error!(repo = %group.repo, "archive agent failed");
        }
    }

    let archive_dest = workspace
        .join("openspec/changes/archive")
        .join(format!("{}-{change}", chrono::Utc::now().format("%Y-%m-%d")));

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
