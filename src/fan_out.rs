use std::path::Path;

use anyhow::Result;
use futures::stream::{self, StreamExt};

use crate::context::{ChangeContext, TempDir};
use crate::engine::{DistributeContext, Engine};
use crate::pipeline::RepoGroup;
use crate::status::TargetState;
use crate::{git, github, status};

/// Result of a single repo group fan-out: per-target state updates.
struct FanOutResult {
    updates: Vec<(String, TargetState, String)>,
}

pub async fn run(
    change: &str, dry_run: bool, concurrency: usize, engine: &dyn Engine, workspace: &Path,
) -> Result<()> {
    let mut ctx = ChangeContext::load(workspace, engine, change)?;
    let groups = ctx.groups()?;

    if dry_run {
        print_dry_run(change, &groups, &ctx);
        return Ok(());
    }

    let pending_groups: Vec<_> = groups
        .into_iter()
        .filter(|group| {
            let all_distributed = group.targets.iter().all(|t| {
                ctx.status
                    .get(&t.id)
                    .map(|s| s.state.is_at_least(status::TargetState::Distributed))
                    .unwrap_or(false)
            });
            if all_distributed {
                tracing::info!(repo = %group.repo, "already distributed, skipping");
            }
            !all_distributed
        })
        .collect();

    if pending_groups.is_empty() {
        ctx.status.print_summary();
        return Ok(());
    }

    let workspace_buf = workspace.to_path_buf();
    let change_str = change.to_string();

    let results: Vec<Result<FanOutResult>> = stream::iter(pending_groups)
        .map(|group| {
            let ws = workspace_buf.clone();
            let ch = change_str.clone();
            async move { fan_out_group(&ch, &group, engine, &ws).await }
        })
        .buffer_unordered(concurrency)
        .collect()
        .await;

    for result in results {
        let outcome = result?;
        for (target_id, new_state, pr_url) in outcome.updates {
            ctx.status.transition(&target_id, new_state)?;
            ctx.status.set_pr(&target_id, pr_url)?;
        }
    }

    ctx.save_status()?;
    println!();
    ctx.status.print_summary();
    Ok(())
}

async fn fan_out_group(
    change: &str, group: &RepoGroup, engine: &dyn Engine, workspace: &Path,
) -> Result<FanOutResult> {
    tracing::info!(repo = %group.repo, crates = ?group.crates, "distributing");

    let tmp = TempDir::new(&group.repo_label())?;

    git::clone_shallow(&group.repo, tmp.path()).await?;
    let branch = group.branch_name(change);
    git::checkout_new_branch(tmp.path(), &branch).await?;

    let dist_ctx = DistributeContext {
        workspace,
        change,
        repo_dir: tmp.path(),
        group,
    };
    engine.distribute(&dist_ctx)?;

    let commit_msg = format!("alc: distribute {change} for {}", group.crates.join(", "));
    git::add_commit_push(tmp.path(), &commit_msg, &branch).await?;

    let (owner, repo_name) = git::parse_repo_url(&group.repo)?;
    let base = git::default_branch(tmp.path()).await?;
    let pr_title = format!("alc: {change} — {}", group.crates.join(", "));
    let pr_body = format!(
        "Distributed from central plan.\n\nTargets: {}\nSpecs: {}",
        group.crates.join(", "),
        group.specs.join(", "),
    );
    let pr_url =
        github::create_draft_pr(&owner, &repo_name, &branch, &base, &pr_title, &pr_body).await?;

    tracing::info!(repo = %group.repo, pr = %pr_url, "distributed");

    let updates = group
        .targets
        .iter()
        .map(|t| (t.id.clone(), TargetState::Distributed, pr_url.clone()))
        .collect();

    Ok(FanOutResult { updates })
}

fn print_dry_run(change: &str, groups: &[RepoGroup], ctx: &ChangeContext) {
    println!("=== DRY RUN: fan-out for '{change}' ===\n");

    let sorted = ctx.pipeline.topological_sort().unwrap_or_default();
    println!("dependency order:");
    for (i, t) in sorted.iter().enumerate() {
        let deps = ctx.pipeline.upstream_of(&t.id);
        if deps.is_empty() {
            println!("  {}. {} (no dependencies)", i + 1, t.id);
        } else {
            println!("  {}. {} (after: {})", i + 1, t.id, deps.join(", "));
        }
    }

    println!("\nrepo groups:");
    for group in groups {
        let branch = group.branch_name(change);
        println!("  {} (branch: {branch}, 1 PR)", group.repo);
        for c in &group.crates {
            println!("    crate: {c}");
        }
        for s in &group.specs {
            println!("    spec:  {s}");
        }
    }
    println!("\nno changes made (dry run)");
}
