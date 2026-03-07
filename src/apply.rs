use std::collections::HashSet;
use std::path::Path;

use anyhow::{Context, Result, bail};
use futures::stream::{self, StreamExt};

use crate::context::{ChangeContext, TempDir};
use crate::engine::Engine;
use crate::pipeline::RepoGroup;
use crate::status::TargetState;
use crate::{agent, brief, git, status};

/// Result of applying a single repo group.
struct ApplyResult {
    repo: String,
    updates: Vec<(String, TargetState)>,
    error: Option<anyhow::Error>,
}

pub async fn run(
    change: &str, target_filter: Option<&str>, dry_run: bool, continue_on_failure: bool,
    concurrency: usize, engine: &dyn Engine, workspace: &Path,
) -> Result<()> {
    let mut ctx = ChangeContext::load(workspace, engine, change)?;
    let levels = ctx.pipeline.dependency_levels(&ctx.registry)?;

    let levels: Vec<Vec<RepoGroup>> = if let Some(filter) = target_filter {
        levels
            .into_iter()
            .map(|level| {
                level
                    .into_iter()
                    .filter(|g| g.targets.iter().any(|t| t.id == filter))
                    .collect()
            })
            .filter(|level: &Vec<RepoGroup>| !level.is_empty())
            .collect()
    } else {
        levels
    };

    let total_groups: usize = levels.iter().map(Vec::len).sum();
    if total_groups == 0 {
        if let Some(filter) = target_filter {
            bail!("target '{filter}' not found in pipeline");
        }
        bail!("no targets in pipeline");
    }

    if dry_run {
        let all_groups: Vec<&RepoGroup> = levels.iter().flat_map(|l| l.iter()).collect();
        print_dry_run(change, &all_groups, engine, &ctx);
        return Ok(());
    }

    let mut had_failure = false;

    for (level_idx, level) in levels.iter().enumerate() {
        let actionable: Vec<&RepoGroup> = level
            .iter()
            .filter(|group| {
                let all_done = group.targets.iter().all(|t| {
                    ctx.status
                        .get(&t.id)
                        .map(|s| s.state.is_at_least(status::TargetState::Implemented))
                        .unwrap_or(false)
                });
                if all_done {
                    tracing::info!(repo = %group.repo, "all targets already implemented, skipping");
                    return false;
                }

                let any_pending = group.targets.iter().any(|t| {
                    ctx.status
                        .get(&t.id)
                        .map(|s| s.state == status::TargetState::Pending)
                        .unwrap_or(true)
                });
                if any_pending {
                    tracing::warn!(repo = %group.repo, "targets in pending state, skipping");
                    return false;
                }

                if is_blocked_by_upstream(group, &ctx) {
                    tracing::warn!(repo = %group.repo, "blocked by upstream, skipping");
                    return false;
                }

                true
            })
            .collect();

        if actionable.is_empty() {
            continue;
        }

        tracing::info!(
            level = level_idx,
            groups = actionable.len(),
            "processing dependency level"
        );

        for group in &actionable {
            for t in &group.targets {
                if !ctx
                    .status
                    .get(&t.id)
                    .map(|s| s.state.is_at_least(status::TargetState::Implemented))
                    .unwrap_or(false)
                {
                    ctx.status.transition(&t.id, TargetState::Applying)?;
                }
            }
        }
        ctx.save_status()?;

        let results: Vec<ApplyResult> = stream::iter(actionable)
            .map(|group| {
                let change = change.to_string();
                async move { apply_group(&change, group, engine).await }
            })
            .buffer_unordered(concurrency)
            .collect()
            .await;

        for result in results {
            for (target_id, new_state) in &result.updates {
                ctx.status.transition(target_id, *new_state)?;
            }

            if let Some(err) = result.error {
                had_failure = true;
                tracing::error!(repo = %result.repo, error = %err, "group failed");
                if !continue_on_failure {
                    ctx.save_status()?;
                    bail!("stopping pipeline: repo '{}' failed: {err}", result.repo);
                }
            }
        }

        ctx.save_status()?;
    }

    println!();
    ctx.status.print_summary();

    if had_failure {
        bail!("one or more repo groups failed (--continue-on-failure was set)");
    }

    Ok(())
}

async fn apply_group(change: &str, group: &RepoGroup, engine: &dyn Engine) -> ApplyResult {
    match apply_group_inner(change, group, engine).await {
        Ok(updates) => ApplyResult {
            repo: group.repo.clone(),
            updates,
            error: None,
        },
        Err(err) => {
            let updates = group
                .targets
                .iter()
                .map(|t| (t.id.clone(), TargetState::Failed))
                .collect();
            ApplyResult {
                repo: group.repo.clone(),
                updates,
                error: Some(err),
            }
        }
    }
}

async fn apply_group_inner(
    change: &str, group: &RepoGroup, engine: &dyn Engine,
) -> Result<Vec<(String, TargetState)>> {
    tracing::info!(repo = %group.repo, crates = ?group.crates, "applying");

    let tmp = TempDir::new(&format!("apply-{}", group.repo_label()))?;
    let branch = group.branch_name(change);
    git::clone_shallow(&group.repo, tmp.path()).await?;
    git::checkout_existing_branch(tmp.path(), &branch)
        .await
        .with_context(|| format!("checking out branch {branch}"))?;

    let change_brief = brief::generate(change, group, engine);
    let apply_cmd = engine.apply_command(change, &change_brief);
    let succeeded = agent::invoke(&apply_cmd, tmp.path()).await?;

    if succeeded {
        let msg = format!("alc: implement {change} for {}", group.crates.join(", "));
        if let Err(e) = git::add_commit_push(tmp.path(), &msg, &branch).await {
            tracing::warn!(repo = %group.repo, error = %e, "push failed (possibly no changes)");
        }

        tracing::info!(repo = %group.repo, "implemented");
        Ok(group
            .targets
            .iter()
            .map(|t| (t.id.clone(), TargetState::Implemented))
            .collect())
    } else {
        bail!("agent failed for repo '{}'", group.repo);
    }
}

/// Check whether any target in this group has an upstream dependency
/// (in another group) that is not yet Implemented.
fn is_blocked_by_upstream(group: &RepoGroup, ctx: &ChangeContext) -> bool {
    let group_target_ids: HashSet<&str> =
        group.targets.iter().map(|t| t.id.as_str()).collect();

    for target in &group.targets {
        for dep in &target.depends_on {
            if group_target_ids.contains(dep.as_str()) {
                continue;
            }
            let met = ctx
                .status
                .get(dep)
                .map(|s| s.state.is_at_least(status::TargetState::Implemented))
                .unwrap_or(false);
            if !met {
                return true;
            }
        }
    }
    false
}

fn print_dry_run(change: &str, groups: &[&RepoGroup], engine: &dyn Engine, ctx: &ChangeContext) {
    println!("=== DRY RUN: apply '{change}' ===\n");

    for group in groups {
        let branch = group.branch_name(change);
        println!("repo: {} (branch: {branch})", group.repo);
        for t in &group.targets {
            let state = ctx
                .status
                .get(&t.id)
                .map(|s| s.state.to_string())
                .unwrap_or_else(|| "unknown".to_string());
            println!("  target: {} (state={})", t.id, state);
        }
        let change_brief = brief::generate(change, group, engine);
        let cmd = engine.apply_command(change, &change_brief);
        println!("  command: {}", cmd.lines().next().unwrap_or(""));
        println!();
    }
    println!("no changes made (dry run)");
}
