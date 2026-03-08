use std::collections::HashSet;

use anyhow::{Context, Result, bail};
use futures::stream::{self, StreamExt};

use crate::context::ChangeContext;
use crate::pipeline::RepoGroup;
use crate::session::Session;
use crate::status::TargetState;
use crate::util::TempDir;
use crate::{agent, brief, git, output, status};

/// Result of applying a single repo group.
struct ApplyResult {
    repo: String,
    updates: Vec<(String, TargetState)>,
    error: Option<anyhow::Error>,
}

#[allow(clippy::too_many_lines)]
pub async fn run(
    change: &str, target_filter: Option<&str>, dry_run: bool, continue_on_failure: bool,
    session: &Session,
) -> Result<()> {
    let mut ctx = ChangeContext::load(session, change)?;
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
        print_dry_run(change, &all_groups, session, &ctx);
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
                        .is_some_and(|s| s.state.is_at_least(status::TargetState::Implemented))
                });
                if all_done {
                    tracing::info!(repo = %group.repo, "all targets already implemented, skipping");
                    return false;
                }

                let any_pending = group.targets.iter().any(|t| {
                    ctx.status
                        .get(&t.id)
                        .is_none_or(|s| s.state == status::TargetState::Pending)
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
                    .is_some_and(|s| s.state.is_at_least(status::TargetState::Implemented))
                {
                    ctx.status.transition(&t.id, TargetState::Applying)?;
                }
            }
        }
        ctx.save_status()?;

        let results: Vec<ApplyResult> = stream::iter(actionable)
            .map(|group| {
                let change = change.to_string();
                async move { apply_group(&change, group, session).await }
            })
            .buffer_unordered(session.concurrency)
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
    output::print_status_summary(&ctx.status);

    if had_failure {
        bail!("one or more repo groups failed (--continue-on-failure was set)");
    }

    Ok(())
}

async fn apply_group(change: &str, group: &RepoGroup, session: &Session) -> ApplyResult {
    match apply_group_inner(change, group, session).await {
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
    change: &str, group: &RepoGroup, session: &Session,
) -> Result<Vec<(String, TargetState)>> {
    tracing::info!(repo = %group.repo, crates = ?group.crates, "applying");

    let tmp = TempDir::new(&format!("apply-{}", group.repo_label()))?;
    let branch = group.branch_name(change);
    git::clone_shallow(&group.repo, tmp.path()).await?;
    git::checkout_existing_branch(tmp.path(), &branch)
        .await
        .with_context(|| format!("checking out branch {branch}"))?;

    let change_brief = brief::generate(change, group, &session.engine);
    let apply_cmd = session.engine.apply_command(change, &change_brief);
    let succeeded = agent::invoke(&apply_cmd, tmp.path()).await?;

    if succeeded {
        let msg = format!("alc: implement {change} for {}", group.crates.join(", "));
        let pushed = git::add_commit_push(tmp.path(), &msg, &branch)
            .await
            .with_context(|| format!("commit/push failed for repo '{}'", group.repo))?;

        if pushed {
            tracing::info!(repo = %group.repo, "implemented and pushed");
        } else {
            tracing::info!(repo = %group.repo, "implemented (no changes to commit)");
        }

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
/// (in another group) that is not yet Implemented. Uses all dependency edges
/// (`depends_on` and `[[dependencies]]`) to stay consistent with topological sort.
fn is_blocked_by_upstream(group: &RepoGroup, ctx: &ChangeContext) -> bool {
    let group_target_ids: HashSet<&str> =
        group.targets.iter().map(|t| t.id.as_str()).collect();

    let all_edges = ctx.pipeline.all_edges();

    for target in &group.targets {
        for (from, to) in &all_edges {
            if *to != target.id {
                continue;
            }
            if group_target_ids.contains(*from) {
                continue;
            }
            let met = ctx
                .status
                .get(from)
                .is_some_and(|s| s.state.is_at_least(status::TargetState::Implemented));
            if !met {
                return true;
            }
        }
    }
    false
}

fn print_dry_run(change: &str, groups: &[&RepoGroup], session: &Session, ctx: &ChangeContext) {
    output::dry_run_banner("apply", change);

    for group in groups {
        let branch = group.branch_name(change);
        println!("repo: {} (branch: {branch})", group.repo);
        for t in &group.targets {
            let state = ctx
                .status
                .get(&t.id)
                .map_or_else(|| "unknown".to_string(), |s| s.state.to_string());
            println!("  target: {} (state={})", t.id, state);
        }
        let change_brief = brief::generate(change, group, &session.engine);
        let cmd = session.engine.apply_command(change, &change_brief);
        println!("  command: {}", cmd.lines().next().unwrap_or(""));
        println!();
    }
    output::dry_run_footer();
}
