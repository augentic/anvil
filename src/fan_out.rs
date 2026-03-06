use std::path::Path;

use anyhow::{Context, Result};

use crate::pipeline::RepoGroup;
use crate::spec_engine::SpecEngine;
use crate::{brief, config, git, pipeline, registry, status};

pub fn run(change: &str, dry_run: bool, engine: &dyn SpecEngine, workspace: &Path) -> Result<()> {
    let change_dir = workspace.join("openspec/changes").join(change);
    let pipeline = pipeline::Pipeline::load(&change_dir.join("pipeline.toml"))?;
    let reg = registry::Registry::load(&workspace.join("registry.toml"))?;
    pipeline.validate(&reg, &change_dir)?;
    let groups = pipeline.group_by_repo(&reg)?;

    let status_path = change_dir.join("status.toml");
    let mut status = status::PipelineStatus::load_or_create(&status_path, change, &pipeline, &reg)?;

    if dry_run {
        print_dry_run(change, &groups, &pipeline);
        return Ok(());
    }

    let lifecycle_source = fetch_lifecycle(&pipeline)?;
    let base_config = workspace.join("openspec/config.yaml");

    for group in &groups {
        let all_distributed = group.targets.iter().all(|t| {
            status
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
        let branch = format!("opsx/{change}");
        git::checkout_new_branch(&tmp, &branch)?;

        engine.init(&tmp)?;
        engine.install_schema(&tmp, &lifecycle_source)?;

        let merged_config = config::generate(engine, &base_config, &reg, group)?;
        std::fs::write(tmp.join("openspec/config.yaml"), &merged_config)
            .context("writing merged config")?;

        let change_brief = brief::generate(change, group);
        engine.create_change(&tmp, &change_brief)?;

        let target_change_dir = engine.change_dir(&tmp, change);
        brief::write(&change_brief, &target_change_dir.join("brief.toml"))?;

        copy_specs(workspace, change, group, &target_change_dir)?;
        copy_upstream(workspace, change, &target_change_dir)?;

        let commit_msg = format!("opsx: distribute {change} specs for {}", group.crates.join(", "));
        git::add_commit_push(&tmp, &commit_msg, &branch)?;

        let pr_title = format!("opsx: {change} — {}", group.crates.join(", "));
        let pr_body = format!(
            "Distributed from central plan.\n\nTargets: {}\nSpecs: {}\n\nRun `/opsx:apply {change}` to implement.",
            group.crates.join(", "),
            group.specs.join(", "),
        );
        let pr_url = git::create_draft_pr(&tmp, &pr_title, &pr_body)?;

        for t in &group.targets {
            status.transition(&t.id, status::TargetState::Distributed)?;
            status.set_pr(&t.id, pr_url.clone())?;
        }
        status.save(&status_path)?;

        tracing::info!(repo = %group.repo, pr = %pr_url, "distributed");
    }

    println!();
    status.print_summary();
    Ok(())
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
        println!("  {} (1 branch, 1 PR)", group.repo);
        for c in &group.crates {
            println!("    crate: {c}");
        }
        for s in &group.specs {
            println!("    spec:  {s}");
        }
    }
    println!("\nno changes made (dry run)");
}

/// Clone the lifecycle repo to a temp directory for schema/template copying.
fn fetch_lifecycle(pipeline: &pipeline::Pipeline) -> Result<std::path::PathBuf> {
    let lifecycle_ref = pipeline.lifecycle_ref.as_deref().unwrap_or("augentic/lifecycle@main");

    let (repo_path, _rev) = lifecycle_ref.split_once('@').unwrap_or((lifecycle_ref, "main"));

    let repo_url = if repo_path.contains("://") || repo_path.starts_with("git@") {
        repo_path.to_string()
    } else {
        format!("https://github.com/{repo_path}.git")
    };

    let tmp = std::env::temp_dir().join(format!("opsx-lifecycle-{}", std::process::id()));
    if tmp.exists() {
        std::fs::remove_dir_all(&tmp)?;
    }

    tracing::info!(repo = %repo_url, "fetching lifecycle schema");
    git::clone_shallow(&repo_url, &tmp)?;

    Ok(tmp)
}

fn tempdir_for_repo(repo_url: &str) -> Result<std::path::PathBuf> {
    let name = repo_url.rsplit('/').next().unwrap_or("repo").trim_end_matches(".git");

    let tmp = std::env::temp_dir().join(format!("opsx-{name}-{}", std::process::id()));
    if tmp.exists() {
        std::fs::remove_dir_all(&tmp)?;
    }
    Ok(tmp)
}

/// Copy spec files from the central change to the target change directory.
fn copy_specs(
    workspace: &Path, change: &str, group: &RepoGroup, target_change_dir: &Path,
) -> Result<()> {
    let central_specs = workspace.join("openspec/changes").join(change).join("specs");
    if !central_specs.exists() {
        return Ok(());
    }

    for spec_name in &group.specs {
        let src = central_specs.join(spec_name);
        let dest = target_change_dir.join("specs").join(spec_name);
        if src.is_dir() {
            copy_dir_recursive(&src, &dest)?;
        } else if src.is_file() {
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(&src, &dest)?;
        }
    }
    Ok(())
}

/// Copy upstream context (architecture, contracts, pipeline.toml) into the target.
fn copy_upstream(workspace: &Path, change: &str, target_change_dir: &Path) -> Result<()> {
    let central = workspace.join("openspec/changes").join(change);
    let upstream = target_change_dir.join("upstream");
    std::fs::create_dir_all(&upstream)?;

    for name in ["architecture.md", "design.md", "contracts.md", "pipeline.toml"] {
        let src = central.join(name);
        if src.exists() {
            std::fs::copy(&src, upstream.join(name))?;
        }
    }

    Ok(())
}

fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<()> {
    std::fs::create_dir_all(dest)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dest_path)?;
        } else {
            std::fs::copy(&src_path, &dest_path)?;
        }
    }
    Ok(())
}
