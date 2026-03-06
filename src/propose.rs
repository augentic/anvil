use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::{agent, pipeline, registry};

pub fn run(change: &str, description: &str, workspace: &Path) -> Result<()> {
    let change_dir = workspace.join("openspec/changes").join(change);
    std::fs::create_dir_all(change_dir.join("specs")).with_context(|| {
        format!("creating change scaffold directories under {}", change_dir.display())
    })?;

    let command = build_propose_prompt(change, description);
    let succeeded = agent::invoke(&command, workspace)?;
    if !succeeded {
        bail!("proposal agent failed for change '{change}'");
    }

    verify_artifacts(&change_dir)?;

    let reg = registry::Registry::load(&workspace.join("registry.toml"))?;
    let pipeline = pipeline::Pipeline::load(&change_dir.join("pipeline.toml"))?;
    pipeline.validate(&reg, &change_dir)?;

    println!("proposal artifacts generated at {}", change_dir.display());
    println!("next step: review artifacts, then run `opsx fan-out {change}`");
    Ok(())
}

fn build_propose_prompt(change: &str, description: &str) -> String {
    format!(
        concat!(
            "Generate OpenSpec planning artifacts for change '{}'.\n\n",
            "User intent:\n",
            "{}\n\n",
            "Write files in this exact directory:\n",
            "openspec/changes/{}\n\n",
            "Required artifact order:\n",
            "1) current-state.md\n",
            "2) proposal.md\n",
            "3) specs/*/spec.md\n",
            "4) design.md\n",
            "5) manifest.md\n",
            "6) pipeline.toml\n\n",
            "Rules:\n",
            "- Use registry.toml to choose impacted targets.\n",
            "- Use openspec/schemas/augentic.yaml and openspec/templates/*.md as guidance.\n",
            "- pipeline.toml must include only targets present in registry.toml.\n",
            "- Include dependency edges when contracts cross targets.\n",
            "- Keep content implementation-ready for distributed apply.\n"
        ),
        change, description, change
    )
}

fn verify_artifacts(change_dir: &Path) -> Result<()> {
    for required in ["current-state.md", "proposal.md", "design.md", "manifest.md", "pipeline.toml"]
    {
        let path = change_dir.join(required);
        if !path.exists() {
            bail!("missing generated artifact: {}", path.display());
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
