use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::engine::Engine;
use crate::{agent, git, registry};

pub fn run(
    change: &str, description: &str, dry_run: bool, engine: &dyn Engine, workspace: &Path,
) -> Result<()> {
    let changes_dir = workspace.join(engine.changes_dir());
    let change_dir = changes_dir.join(change);

    if change_dir.exists() {
        bail!("change '{}' already exists at {}", change, change_dir.display());
    }

    std::fs::create_dir_all(change_dir.join("specs")).with_context(|| {
        format!("creating change scaffold under {}", change_dir.display())
    })?;

    let reg = registry::Registry::load(&workspace.join("registry.toml"))?;
    let context = gather_context(workspace, engine, &reg)?;

    let prompt = engine.propose_prompt(change, description, &context);

    if dry_run {
        println!("=== DRY RUN: propose '{change}' ===\n");
        println!("change dir: {}\n", change_dir.display());
        println!("--- AGENT PROMPT ---\n{prompt}\n--- END ---");
        std::fs::remove_dir_all(&change_dir)?;
        return Ok(());
    }

    let succeeded = agent::invoke(&prompt, workspace)?;
    if !succeeded {
        bail!("proposal agent failed for change '{change}'");
    }

    verify_artifacts(&change_dir, engine)?;

    println!("planning artefacts generated at {}", change_dir.display());
    println!("next step: review artefacts, then run `lc fan-out {change}`");
    Ok(())
}

/// Gather platform context for the propose prompt:
/// registry summary + current specs from target repos.
fn gather_context(workspace: &Path, engine: &dyn Engine, reg: &registry::Registry) -> Result<String> {
    let mut ctx = String::from("=== REGISTRY ===\n");
    for svc in &reg.services {
        ctx.push_str(&format!(
            "- {} (repo={}, crate={}, domain={}, caps=[{}])\n",
            svc.id,
            svc.repo,
            svc.crate_name,
            svc.domain,
            svc.capabilities.join(", "),
        ));
    }

    ctx.push_str("\n=== CURRENT SPECS ===\n");

    let mut seen_repos: std::collections::HashSet<String> = std::collections::HashSet::new();

    for svc in &reg.services {
        if !seen_repos.insert(svc.repo.clone()) {
            continue;
        }

        let repo_specs = try_read_repo_specs(workspace, &svc.repo, engine);
        if let Some(specs_content) = repo_specs {
            ctx.push_str(&format!("\n--- repo: {} ---\n{}\n", svc.repo, specs_content));
        }
    }

    Ok(ctx)
}

/// Try to read specs from a target repo. Looks for the repo as a sibling
/// directory first (workspace layout), otherwise clones shallowly.
fn try_read_repo_specs(workspace: &Path, repo_url: &str, engine: &dyn Engine) -> Option<String> {
    let repo_name = repo_url.rsplit('/').next().unwrap_or("repo").trim_end_matches(".git");

    if let Some(parent) = workspace.parent() {
        let sibling = parent.join(repo_name);
        let specs_dir = sibling.join(engine.specs_dir());
        if specs_dir.is_dir() {
            return Some(read_specs_dir(&specs_dir));
        }
    }

    let tmp = std::env::temp_dir().join(format!("opsx-specs-{repo_name}-{}", std::process::id()));
    if tmp.exists() {
        let _ = std::fs::remove_dir_all(&tmp);
    }

    if git::clone_shallow(repo_url, &tmp).is_ok() {
        let specs_dir = tmp.join(engine.specs_dir());
        let result = if specs_dir.is_dir() {
            Some(read_specs_dir(&specs_dir))
        } else {
            None
        };
        let _ = std::fs::remove_dir_all(&tmp);
        return result;
    }

    None
}

fn read_specs_dir(dir: &Path) -> String {
    let mut output = String::new();
    if let Ok(entries) = collect_md_files(dir) {
        for path in entries {
            let rel = path.strip_prefix(dir).unwrap_or(&path);
            if let Ok(content) = std::fs::read_to_string(&path) {
                output.push_str(&format!("\n## {}\n{}\n", rel.display(), content));
            }
        }
    }
    output
}

fn collect_md_files(dir: &Path) -> Result<Vec<std::path::PathBuf>> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            files.extend(collect_md_files(&path)?);
        } else if path.extension().is_some_and(|e| e == "md") {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

fn verify_artifacts(change_dir: &Path, engine: &dyn Engine) -> Result<()> {
    for required in engine.required_artifacts() {
        let path = change_dir.join(required);
        if !path.exists() {
            bail!("missing generated artefact: {}", path.display());
        }
    }

    let specs_dir = change_dir.join("specs");
    let has_specs = specs_dir.exists()
        && std::fs::read_dir(&specs_dir)
            .with_context(|| format!("reading {}", specs_dir.display()))?
            .any(|entry| entry.is_ok());
    if !has_specs {
        bail!("specs directory is empty after propose: {}", specs_dir.display());
    }

    Ok(())
}
