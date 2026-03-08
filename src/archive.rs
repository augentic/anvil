use anyhow::{Context, Result, bail};

use crate::context::ChangeContext;
use crate::session::Session;
use crate::{git, github, output, status};

pub async fn run(change: &str, mark_ready: bool, session: &Session) -> Result<()> {
    let mut ctx = ChangeContext::load(session, change)?;
    let gh = session.github()?;
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

        let mut info = github::pull_request_info(gh, &owner, &repo_name, pr_number)
            .await
            .with_context(|| format!("reading PR metadata for target {id}"))?;

        if mark_ready {
            let current = ctx.status.get(&id).context("target missing from status")?;
            if current.state == status::TargetState::Implemented
                && info.state.eq_ignore_ascii_case("OPEN")
                && info.is_draft
            {
                github::mark_pr_ready(gh, &owner, &repo_name, pr_number)
                    .await
                    .with_context(|| format!("marking PR ready for target {id}"))?;
                info = github::pull_request_info(gh, &owner, &repo_name, pr_number)
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

    output::print_status_summary(&ctx.status);

    let all_merged = ctx
        .status
        .targets
        .iter()
        .all(|t| t.state.is_at_least(status::TargetState::Merged));

    if all_merged {
        archive(change, &ctx, session)?;
    }

    Ok(())
}

fn archive(change: &str, ctx: &ChangeContext, session: &Session) -> Result<()> {
    let archive_dest = session
        .workspace
        .join(session.engine.archive_dir())
        .join(session.engine.archive_dirname(change));

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
