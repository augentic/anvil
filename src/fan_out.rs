use std::path::Path;

use anyhow::Result;

use crate::engine::{DistributeContext, Engine};
use crate::pipeline::RepoGroup;
use crate::{git, pipeline, registry, status};

pub fn run(change: &str, dry_run: bool, engine: &dyn Engine, workspace: &Path) -> Result<()> {
    let change_dir = workspace.join(engine.changes_dir()).join(change);
    let pipeline = pipeline::Pipeline::load(&change_dir.join("pipeline.toml"))?;
    let reg = registry::Registry::load(&workspace.join("registry.toml"))?;
    pipeline.validate(&reg, &change_dir)?;
    let groups = pipeline.group_by_repo(&reg)?;

    let status_path = change_dir.join("status.toml");
    let mut pstatus =
        status::PipelineStatus::load_or_create(&status_path, change, &pipeline, &reg)?;

    if dry_run {
        print_dry_run(change, &groups, &pipeline);
        return Ok(());
    }

    for group in &groups {
        let all_distributed = group.targets.iter().all(|t| {
            pstatus
                .get(&t.id)
                .map(|s| s.state.is_at_least(status::TargetState::Distributed))
                .unwrap_or(false)
        });
        if all_distributed {
            tracing::info!(repo = %group.repo, "already distributed, skipping");
            continue;
        }

        tracing::info!(repo = %group.repo, crates = ?group.crates, "distributing");

        let tmp = tempdir_for_repo(&group.repo)?;

        git::clone_shallow(&group.repo, &tmp)?;
        let branch = branch_name(change, group);
        git::checkout_new_branch(&tmp, &branch)?;

        let ctx = DistributeContext {
            workspace,
            change,
            repo_dir: &tmp,
            group,
        };
        engine.distribute(&ctx)?;

        let commit_msg =
            format!("lc: distribute {change} for {}", group.crates.join(", "));
        git::add_commit_push(&tmp, &commit_msg, &branch)?;

        let pr_title = format!("lc: {change} — {}", group.crates.join(", "));
        let pr_body = format!(
            "Distributed from central plan.\n\nTargets: {}\nSpecs: {}",
            group.crates.join(", "),
            group.specs.join(", "),
        );
        let pr_url = git::create_draft_pr(&tmp, &pr_title, &pr_body)?;

        for t in &group.targets {
            pstatus.transition(&t.id, status::TargetState::Distributed)?;
            pstatus.set_pr(&t.id, pr_url.clone())?;
        }
        pstatus.save(&status_path)?;

        tracing::info!(repo = %group.repo, pr = %pr_url, "distributed");
    }

    println!();
    pstatus.print_summary();
    Ok(())
}

fn branch_name(change: &str, group: &RepoGroup) -> String {
    group
        .targets
        .first()
        .and_then(|t| t.branch.as_deref())
        .map(String::from)
        .unwrap_or_else(|| format!("lc/{change}"))
}

fn print_dry_run(change: &str, groups: &[RepoGroup], pipeline: &pipeline::Pipeline) {
    println!("=== DRY RUN: fan-out for '{change}' ===\n");

    let sorted = pipeline.topological_sort().unwrap_or_default();
    println!("dependency order:");
    for (i, t) in sorted.iter().enumerate() {
        let deps = pipeline.upstream_of(&t.id);
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

fn tempdir_for_repo(repo_url: &str) -> Result<std::path::PathBuf> {
    let name = repo_url.rsplit('/').next().unwrap_or("repo").trim_end_matches(".git");

    let tmp = std::env::temp_dir().join(format!("opsx-{name}-{}", std::process::id()));
    if tmp.exists() {
        std::fs::remove_dir_all(&tmp)?;
    }
    Ok(tmp)
}
