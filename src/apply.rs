use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::context::{self, ChangeContext};
use crate::engine::Engine;
use crate::pipeline::RepoGroup;
use crate::{agent, brief, git, status};

pub fn run(
    change: &str, target_filter: Option<&str>, dry_run: bool, engine: &dyn Engine,
    workspace: &Path,
) -> Result<()> {
    let mut ctx = ChangeContext::load(workspace, engine, change)?;
    let groups = ctx.pipeline.groups_in_dependency_order(&ctx.registry)?;

    let groups: Vec<_> = if let Some(filter) = target_filter {
        groups
            .into_iter()
            .filter(|g| g.targets.iter().any(|t| t.id == filter))
            .collect()
    } else {
        groups
    };

    if groups.is_empty() {
        if let Some(filter) = target_filter {
            bail!("target '{filter}' not found in pipeline");
        }
        bail!("no targets in pipeline");
    }

    if dry_run {
        print_dry_run(change, &groups, engine, &ctx);
        return Ok(());
    }

    for group in &groups {
        let all_done = group.targets.iter().all(|t| {
            ctx.status
                .get(&t.id)
                .map(|s| s.state.is_at_least(status::TargetState::Implemented))
                .unwrap_or(false)
        });
        if all_done {
            tracing::info!(repo = %group.repo, "all targets already implemented, skipping");
            continue;
        }

        let any_pending = group.targets.iter().any(|t| {
            ctx.status
                .get(&t.id)
                .map(|s| s.state == status::TargetState::Pending)
                .unwrap_or(true)
        });
        if any_pending {
            bail!(
                "repo '{}' has targets in pending state — run `alc fan-out {}` first",
                group.repo,
                change
            );
        }

        if is_blocked_by_upstream(group, &ctx) {
            tracing::warn!(repo = %group.repo, "blocked by incomplete upstream dependencies, skipping");
            continue;
        }

        tracing::info!(repo = %group.repo, crates = ?group.crates, "applying");

        for t in &group.targets {
            if !ctx
                .status
                .get(&t.id)
                .map(|s| s.state.is_at_least(status::TargetState::Implemented))
                .unwrap_or(false)
            {
                ctx.status
                    .transition(&t.id, status::TargetState::Applying)?;
            }
        }
        ctx.save_status()?;

        let tmp = context::temp_dir(&format!("apply-{}", repo_label(&group.repo)))?;
        let branch = branch_name(change, group);
        git::clone_shallow(&group.repo, &tmp)?;
        git::run_cmd("git", &["checkout", &branch], &tmp)
            .with_context(|| format!("checking out branch {branch}"))?;

        let change_brief = brief::generate(change, group, engine);
        let apply_cmd = engine.apply_command(change, &change_brief);
        let succeeded = agent::invoke(&apply_cmd, &tmp)?;

        if succeeded {
            let msg = format!("alc: implement {change} for {}", group.crates.join(", "));
            if let Err(e) = git::add_commit_push(&tmp, &msg, &branch) {
                tracing::warn!(repo = %group.repo, error = %e, "push failed (possibly no changes)");
            }

            for t in &group.targets {
                ctx.status
                    .transition(&t.id, status::TargetState::Implemented)?;
            }
            tracing::info!(repo = %group.repo, "implemented");
        } else {
            for t in &group.targets {
                ctx.status
                    .transition(&t.id, status::TargetState::Failed)?;
            }
            tracing::error!(repo = %group.repo, "agent failed");
            ctx.save_status()?;
            bail!("stopping pipeline: repo '{}' failed", group.repo);
        }

        ctx.save_status()?;
    }

    println!();
    ctx.status.print_summary();
    Ok(())
}

/// Check whether any target in this group has an upstream dependency
/// (in another group) that is not yet Implemented.
fn is_blocked_by_upstream(group: &RepoGroup, ctx: &ChangeContext) -> bool {
    let group_target_ids: std::collections::HashSet<&str> =
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

fn branch_name(change: &str, group: &RepoGroup) -> String {
    group
        .targets
        .first()
        .and_then(|t| t.branch.as_deref())
        .map(String::from)
        .unwrap_or_else(|| format!("alc/{change}"))
}

fn repo_label(repo_url: &str) -> String {
    repo_url
        .rsplit('/')
        .next()
        .unwrap_or("repo")
        .trim_end_matches(".git")
        .to_string()
}

fn print_dry_run(change: &str, groups: &[RepoGroup], engine: &dyn Engine, ctx: &ChangeContext) {
    println!("=== DRY RUN: apply '{change}' ===\n");

    for group in groups {
        let branch = branch_name(change, group);
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
