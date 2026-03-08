use std::fmt::Write as _;
use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::session::Session;
use crate::util::TempDir;
use crate::{agent, git, output, registry};

#[allow(clippy::similar_names)]
pub async fn run(change: &str, description: &str, dry_run: bool, session: &Session) -> Result<()> {
    let changes_dir = session.workspace.join(session.engine.changes_dir());
    let change_dir = changes_dir.join(change);

    if change_dir.exists() {
        bail!("change '{}' already exists at {}", change, change_dir.display());
    }

    std::fs::create_dir_all(change_dir.join("specs")).with_context(|| {
        format!("creating change scaffold under {}", change_dir.display())
    })?;

    let reg = registry::Registry::load(&session.workspace.join("registry.toml"))?;
    let context = gather_context(session, &reg).await?;

    let prompt = session.engine.propose_prompt(change, description, &context);

    if dry_run {
        output::dry_run_banner("propose", change);
        println!("change dir: {}\n", change_dir.display());
        println!("--- AGENT PROMPT ---\n{prompt}\n--- END ---");
        output::dry_run_footer();
        std::fs::remove_dir_all(&change_dir)?;
        return Ok(());
    }

    let succeeded = agent::invoke(&prompt, &session.workspace).await?;
    if !succeeded {
        bail!("proposal agent failed for change '{change}'");
    }

    verify_artifacts(&change_dir, session)?;

    println!("planning artefacts generated at {}", change_dir.display());
    println!("next step: review artefacts, then run `alc fan-out {change}`");
    Ok(())
}

/// Gather platform context for the propose prompt:
/// registry summary + current specs from target repos.
async fn gather_context(session: &Session, reg: &registry::Registry) -> Result<String> {
    let mut ctx = String::from("=== REGISTRY ===\n");
    for svc in &reg.services {
        let _ = writeln!(
            ctx,
            "- {} (repo={}, crate={}, domain={}, caps=[{}])",
            svc.id,
            svc.repo,
            svc.crate_name,
            svc.domain,
            svc.capabilities.join(", "),
        );
    }

    ctx.push_str("\n=== CURRENT SPECS ===\n");

    let mut seen_repos: std::collections::HashSet<String> = std::collections::HashSet::new();

    for svc in &reg.services {
        if !seen_repos.insert(svc.repo.clone()) {
            continue;
        }

        let repo_specs = try_read_repo_specs(session, &svc.repo).await;
        if let Some(specs_content) = repo_specs {
            let _ = write!(ctx, "\n--- repo: {} ---\n{specs_content}\n", svc.repo);
        }
    }

    Ok(ctx)
}

/// Try to read specs from a target repo. Looks for the repo as a sibling
/// directory first (workspace layout), otherwise clones shallowly.
async fn try_read_repo_specs(session: &Session, repo_url: &str) -> Option<String> {
    let repo_name = git::repo_name(repo_url);

    if let Some(parent) = session.workspace.parent() {
        let sibling = parent.join(repo_name);
        let specs_dir = sibling.join(session.engine.specs_dir());
        if specs_dir.is_dir() {
            match read_specs_dir(&specs_dir) {
                Ok(content) => return Some(content),
                Err(e) => {
                    tracing::warn!(repo = repo_url, error = %e, "failed to read specs from sibling");
                }
            }
        }
    }

    let tmp = match TempDir::new(&format!("specs-{repo_name}")) {
        Ok(t) => t,
        Err(e) => {
            tracing::warn!(repo = repo_url, error = %e, "failed to create temp dir for specs");
            return None;
        }
    };

    if git::clone_shallow(repo_url, tmp.path()).await.is_ok() {
        let specs_dir = tmp.path().join(session.engine.specs_dir());
        if specs_dir.is_dir() {
            match read_specs_dir(&specs_dir) {
                Ok(content) => return Some(content),
                Err(e) => {
                    tracing::warn!(repo = repo_url, error = %e, "failed to read specs from clone");
                }
            }
        }
    }

    None
}

fn read_specs_dir(dir: &Path) -> Result<String> {
    let mut output = String::new();
    let entries = collect_md_files(dir)
        .with_context(|| format!("collecting spec files from {}", dir.display()))?;
    for path in entries {
        let rel = path.strip_prefix(dir).unwrap_or(&path);
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("reading spec file {}", path.display()))?;
        let _ = write!(output, "\n## {}\n{content}\n", rel.display());
    }
    Ok(output)
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

fn verify_artifacts(change_dir: &Path, session: &Session) -> Result<()> {
    for required in session.engine.required_artifacts() {
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
