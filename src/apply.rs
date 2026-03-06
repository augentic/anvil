use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::spec_engine::SpecEngine;
use crate::{agent, git, pipeline, registry, status};

pub fn run(
    change: &str, target_filter: Option<&str>, engine: &dyn SpecEngine, workspace: &Path,
) -> Result<()> {
    let change_dir = workspace.join("openspec/changes").join(change);
    let pipeline = pipeline::Pipeline::load(&change_dir.join("pipeline.toml"))?;
    let reg = registry::Registry::load(&workspace.join("registry.toml"))?;
    pipeline.validate(&reg, &change_dir)?;

    let status_path = change_dir.join("status.toml");
    let mut pstatus =
        status::PipelineStatus::load_or_create(&status_path, change, &pipeline, &reg)?;

    let sorted = pipeline.topological_sort()?;

    let targets: Vec<_> = if let Some(filter) = target_filter {
        sorted.into_iter().filter(|t| t.id == filter).collect()
    } else {
        sorted
    };

    if targets.is_empty() {
        if let Some(filter) = target_filter {
            bail!("target '{filter}' not found in pipeline");
        }
        bail!("no targets in pipeline");
    }

    for target in &targets {
        let ts = pstatus.get(&target.id).context("target missing from status")?;

        if ts.state.is_at_least(status::TargetState::Implemented) {
            tracing::info!(target = %target.id, state = %ts.state, "already implemented, skipping");
            continue;
        }

        if ts.state == status::TargetState::Pending {
            bail!(
                "target '{}' is in pending state — run `opsx fan-out {}` first",
                target.id,
                change
            );
        }

        let upstream_ids = pipeline.upstream_of(&target.id);
        let blocked = upstream_ids.iter().any(|dep_id| {
            !pstatus
                .get(dep_id)
                .map(|s| s.state.is_at_least(status::TargetState::Implemented))
                .unwrap_or(false)
        });

        if blocked && pipeline.stop_on_failure() {
            tracing::warn!(
                target = %target.id,
                deps = ?upstream_ids,
                "blocked by incomplete dependencies, skipping"
            );
            continue;
        }

        tracing::info!(target = %target.id, "applying");
        pstatus.transition(&target.id, status::TargetState::Applying)?;
        pstatus.save(&status_path)?;

        let svc = reg.find_by_id(&target.id).context("target not in registry")?;

        let tmp =
            std::env::temp_dir().join(format!("opsx-apply-{}-{}", target.id, std::process::id()));
        if tmp.exists() {
            std::fs::remove_dir_all(&tmp)?;
        }

        let branch = format!("opsx/{change}");
        git::clone_shallow(&svc.repo, &tmp)?;
        git::run_cmd("git", &["checkout", &branch], &tmp)
            .with_context(|| format!("checking out branch {branch}"))?;

        let apply_cmd = engine.agent_apply_command(change);
        let succeeded = agent::invoke(&apply_cmd, &tmp)?;

        if succeeded {
            let msg = format!("opsx: implement {change} for {}", target.id);
            if let Err(e) = git::add_commit_push(&tmp, &msg, &branch) {
                tracing::warn!(target = %target.id, error = %e, "push failed (possibly no changes)");
            }

            pstatus.transition(&target.id, status::TargetState::Implemented)?;
            tracing::info!(target = %target.id, "implemented");
        } else {
            pstatus.transition(&target.id, status::TargetState::Failed)?;
            tracing::error!(target = %target.id, "agent failed");
            if pipeline.stop_on_failure() {
                pstatus.save(&status_path)?;
                bail!("stopping pipeline: target '{}' failed", target.id);
            }
        }

        pstatus.save(&status_path)?;
    }

    println!();
    pstatus.print_summary();
    Ok(())
}
