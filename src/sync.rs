use std::path::Path;

use anyhow::{Context, Result};

use crate::context::ChangeContext;
use crate::engine::Engine;
use crate::{git, github, status};

pub async fn run(
    change: &str, mark_ready: bool, engine: &dyn Engine, workspace: &Path,
) -> Result<()> {
    let mut ctx = ChangeContext::load(workspace, engine, change)?;
    let mut changed = false;

    let targets: Vec<(String, Option<String>)> = ctx
        .status
        .targets
        .iter()
        .map(|t| (t.id.clone(), t.pr.clone()))
        .collect();

    for (id, pr_opt) in targets {
        let Some(pr_url) = pr_opt else {
            tracing::debug!(target = %id, "no PR URL, skipping sync");
            continue;
        };

        let (owner, repo_name, pr_number) = git::parse_pr_url(&pr_url)
            .with_context(|| format!("parsing PR URL for target {id}"))?;

        let mut info = github::pull_request_info(&owner, &repo_name, pr_number)
            .await
            .with_context(|| format!("reading PR metadata for target {id}"))?;

        if mark_ready {
            let current = ctx.status.get(&id).context("target missing from status")?;
            if current.state == status::TargetState::Implemented
                && info.state.eq_ignore_ascii_case("OPEN")
                && info.is_draft
            {
                github::mark_pr_ready(&owner, &repo_name, pr_number)
                    .await
                    .with_context(|| format!("marking PR ready for target {id}"))?;
                info = github::pull_request_info(&owner, &repo_name, pr_number)
                    .await
                    .with_context(|| {
                        format!("re-reading PR metadata after ready for target {id}")
                    })?;
            }
        }

        let state = ctx
            .status
            .get(&id)
            .context("target missing from status")?
            .state;
        if info.merged_at.is_some() || info.state.eq_ignore_ascii_case("MERGED") {
            if !state.is_at_least(status::TargetState::Merged) {
                ctx.status.transition(&id, status::TargetState::Merged)?;
                changed = true;
            }
            continue;
        }

        if info.state.eq_ignore_ascii_case("OPEN") {
            if !info.is_draft && state == status::TargetState::Implemented {
                ctx.status
                    .transition(&id, status::TargetState::Reviewing)?;
                changed = true;
            }
            continue;
        }

        if info.state.eq_ignore_ascii_case("CLOSED") && state != status::TargetState::Failed {
            ctx.status.transition(&id, status::TargetState::Failed)?;
            changed = true;
        }
    }

    if changed {
        ctx.save_status()?;
    }

    ctx.status.print_summary();
    Ok(())
}
