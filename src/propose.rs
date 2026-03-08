use std::fmt::Write as _;
use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::session::Session;
use crate::{agent, output, registry};

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
    let context = gather_context(session, &reg)?;

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
    println!("next step: review artefacts, then run `alc apply {change}`");
    Ok(())
}

/// Gather platform context for the propose prompt:
/// registry summary + domain docs from the local `domains/` directory.
fn gather_context(session: &Session, reg: &registry::Registry) -> Result<String> {
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

    let domains_dir = session.workspace.join(session.engine.domains_dir());
    if domains_dir.is_dir() {
        ctx.push_str("\n=== DOMAINS ===\n");
        let mut domains = read_domain_dirs(&domains_dir)?;
        domains.sort_by(|(a, _), (b, _)| a.cmp(b));
        for (name, content) in domains {
            let _ = write!(ctx, "\n--- domain: {name} ---\n{content}\n");
        }
    }

    Ok(ctx)
}

/// Read all domain subdirectories, returning `(domain_name, concatenated_md_content)` pairs.
fn read_domain_dirs(domains_dir: &Path) -> Result<Vec<(String, String)>> {
    let mut domains = Vec::new();
    for entry in std::fs::read_dir(domains_dir)
        .with_context(|| format!("reading {}", domains_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        let files = collect_md_files(&path)
            .with_context(|| format!("collecting docs from domain '{name}'"))?;

        let mut content = String::new();
        for file in files {
            let rel = file.strip_prefix(&path).unwrap_or(&file);
            let text = std::fs::read_to_string(&file)
                .with_context(|| format!("reading {}", file.display()))?;
            let _ = write!(content, "\n## {}\n{text}\n", rel.display());
        }

        if !content.is_empty() {
            domains.push((name, content));
        }
    }
    Ok(domains)
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
