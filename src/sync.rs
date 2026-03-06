use std::path::Path;

use anyhow::{Context, Result};

use crate::{git, pipeline, registry, status};

pub fn run(change: &str, mark_ready: bool, workspace: &Path) -> Result<()> {
    let change_dir = workspace.join("openspec/changes").join(change);
    let pipeline = pipeline::Pipeline::load(&change_dir.join("pipeline.toml"))?;
    let reg = registry::Registry::load(&workspace.join("registry.toml"))?;
    pipeline.validate(&reg, &change_dir)?;

    let status_path = change_dir.join("status.toml");
    let mut pstatus =
        status::PipelineStatus::load_or_create(&status_path, change, &pipeline, &reg)?;
    let mut changed = false;

    let targets: Vec<(String, Option<String>)> =
        pstatus.targets.iter().map(|t| (t.id.clone(), t.pr.clone())).collect();

    for (id, pr_opt) in targets {
        let Some(pr_url) = pr_opt else {
            tracing::debug!(target = %id, "no PR URL, skipping sync");
            continue;
        };

        let mut info = git::pull_request_info(&pr_url, workspace)
            .with_context(|| format!("reading PR metadata for target {id}"))?;

        if mark_ready {
            let current = pstatus.get(&id).context("target missing from status")?;
            if current.state == status::TargetState::Implemented
                && info.state.eq_ignore_ascii_case("OPEN")
                && info.is_draft
            {
                git::mark_pr_ready(&pr_url, workspace)
                    .with_context(|| format!("marking PR ready for target {id}"))?;
                info = git::pull_request_info(&pr_url, workspace).with_context(|| {
                    format!("re-reading PR metadata after ready for target {id}")
                })?;
            }
        }

        let state = pstatus.get(&id).context("target missing from status")?.state;
        if info.merged_at.is_some() || info.state.eq_ignore_ascii_case("MERGED") {
            if !state.is_at_least(status::TargetState::Merged) {
                pstatus.transition(&id, status::TargetState::Merged)?;
                changed = true;
            }
            continue;
        }

        if info.state.eq_ignore_ascii_case("OPEN") {
            if !info.is_draft && state == status::TargetState::Implemented {
                pstatus.transition(&id, status::TargetState::Reviewing)?;
                changed = true;
            }
            continue;
        }

        if info.state.eq_ignore_ascii_case("CLOSED") && state != status::TargetState::Failed {
            pstatus.transition(&id, status::TargetState::Failed)?;
            changed = true;
        }
    }

    if changed {
        pstatus.save(&status_path)?;
    }

    pstatus.print_summary();
    Ok(())
}
