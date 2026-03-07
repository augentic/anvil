use std::path::Path;

use anyhow::Result;

use crate::context::{self, ChangeContext};
use crate::engine::{DistributeContext, Engine};
use crate::pipeline::RepoGroup;
use crate::{git, status};

pub fn run(change: &str, dry_run: bool, engine: &dyn Engine, workspace: &Path) -> Result<()> {
    let mut ctx = ChangeContext::load(workspace, engine, change)?;
    let groups = ctx.groups()?;

    if dry_run {
        print_dry_run(change, &groups, &ctx);
        return Ok(());
    }

    for group in &groups {
        let all_distributed = group.targets.iter().all(|t| {
            ctx.status
                .get(&t.id)
                .map(|s| s.state.is_at_least(status::TargetState::Distributed))
                .unwrap_or(false)
        });
        if all_distributed {
            tracing::info!(repo = %group.repo, "already distributed, skipping");
            continue;
        }

        tracing::info!(repo = %group.repo, crates = ?group.crates, "distributing");

        let tmp = context::temp_dir(&repo_label(&group.repo))?;

        git::clone_shallow(&group.repo, &tmp)?;
        let branch = branch_name(change, group);
        git::checkout_new_branch(&tmp, &branch)?;

        let dist_ctx = DistributeContext {
            workspace,
            change,
            repo_dir: &tmp,
            group,
        };
        engine.distribute(&dist_ctx)?;

        let commit_msg = format!("alc: distribute {change} for {}", group.crates.join(", "));
        git::add_commit_push(&tmp, &commit_msg, &branch)?;

        let pr_title = format!("alc: {change} — {}", group.crates.join(", "));
        let pr_body = format!(
            "Distributed from central plan.\n\nTargets: {}\nSpecs: {}",
            group.crates.join(", "),
            group.specs.join(", "),
        );
        let pr_url = git::create_draft_pr(&tmp, &pr_title, &pr_body)?;

        for t in &group.targets {
            ctx.status.transition(&t.id, status::TargetState::Distributed)?;
            ctx.status.set_pr(&t.id, pr_url.clone())?;
        }
        ctx.save_status()?;

        let _ = std::fs::remove_dir_all(&tmp);
        tracing::info!(repo = %group.repo, pr = %pr_url, "distributed");
    }

    println!();
    ctx.status.print_summary();
    Ok(())
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
        let branch = branch_name(change, group);
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
