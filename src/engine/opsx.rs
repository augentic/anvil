use std::path::Path;

use anyhow::{Context, Result};

use super::{DistributeContext, Engine, UpstreamPaths};
use crate::brief::{self, ChangeBrief};

pub struct OpsxEngine;

impl Engine for OpsxEngine {
    fn name(&self) -> &str {
        "opsx"
    }

    fn specs_dir(&self) -> &str {
        "openspec/specs"
    }

    fn changes_dir(&self) -> &str {
        "openspec/changes"
    }

    fn archive_dir(&self) -> &str {
        "openspec/changes/archive"
    }

    fn propose_prompt(&self, change: &str, description: &str, context: &str) -> String {
        format!(
            concat!(
                "Generate planning artefacts for change '{change}'.\n\n",
                "Description:\n{description}\n\n",
                "Platform context:\n{context}\n\n",
                "Write files under: openspec/changes/{change}/\n\n",
                "Required artefacts (in order):\n",
                "1) proposal.md — why + what, per-affected-service summary\n",
                "2) specs/<service-id>/<capability>/spec.md — delta specs (ADDED/MODIFIED/REMOVED)\n",
                "3) design.md — technical design per service with enough detail for review\n",
                "4) tasks.md — implementation tasks grouped by '## Repo: <name>' headers\n",
                "5) pipeline.toml — execution config with [[targets]] and [[dependencies]]\n\n",
                "Rules:\n",
                "- Read registry.toml to identify impacted services\n",
                "- Read openspec/specs/ from affected target repos for current state\n",
                "- Namespace delta specs under specs/<service-id>/<capability>/\n",
                "- pipeline.toml targets must reference services in registry.toml\n",
                "- Include dependency edges when contracts cross services\n",
                "- proposal.md and design.md must be detailed enough for centralised review\n",
            ),
            change = change,
            description = description,
            context = context,
        )
    }

    fn required_artifacts(&self) -> Vec<&str> {
        vec!["proposal.md", "design.md", "tasks.md", "pipeline.toml"]
    }

    fn distribute(&self, ctx: &DistributeContext) -> Result<()> {
        let change_brief = brief::generate(ctx.change, ctx.group, self);

        let opsx_dir = ctx.repo_dir.join(".opsx");
        std::fs::create_dir_all(&opsx_dir).context("creating .opsx dir")?;

        brief::write(&change_brief, &opsx_dir.join("brief.toml"))?;

        copy_specs(ctx.workspace, ctx.change, ctx.group, &opsx_dir)?;
        copy_upstream(ctx.workspace, ctx.change, &opsx_dir)?;

        Ok(())
    }

    fn apply_command(&self, change: &str, brief: &ChangeBrief) -> String {
        let crates = brief.target.crates.join(", ");
        let specs: Vec<_> = brief.specs.files.iter().map(String::as_str).collect();
        format!(
            concat!(
                "Implement change '{change}' in this repository.\n\n",
                "Target crates: {crates}\n",
                "Delta specs: {specs}\n\n",
                "Instructions:\n",
                "1. Read .opsx/brief.toml for change context\n",
                "2. Read .opsx/upstream/design.md for technical design\n",
                "3. Read .opsx/upstream/tasks.md for implementation tasks\n",
                "4. Read .opsx/specs/ for delta spec requirements\n",
                "5. Implement code changes per the tasks\n",
                "6. Update openspec/specs/ to reflect the new behaviour\n",
            ),
            change = change,
            crates = crates,
            specs = specs.join(", "),
        )
    }

    fn spec_file_path(&self, spec_name: &str) -> String {
        format!("{spec_name}/spec.md")
    }

    fn upstream_paths(&self) -> UpstreamPaths {
        UpstreamPaths {
            design: "upstream/design.md",
            tasks: "upstream/tasks.md",
            pipeline: "upstream/pipeline.toml",
        }
    }

    fn archive_dirname(&self, change: &str) -> String {
        format!("{}-{change}", chrono::Utc::now().format("%Y-%m-%d"))
    }
}

use crate::pipeline::RepoGroup;

fn copy_specs(workspace: &Path, change: &str, group: &RepoGroup, dest_dir: &Path) -> Result<()> {
    let central_specs = OpsxEngine.change_dir(workspace, change).join("specs");
    if !central_specs.exists() {
        return Ok(());
    }

    for spec_name in &group.specs {
        let src = central_specs.join(spec_name);
        let dest = dest_dir.join("specs").join(spec_name);
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

fn copy_upstream(workspace: &Path, change: &str, dest_dir: &Path) -> Result<()> {
    let central = OpsxEngine.change_dir(workspace, change);
    let upstream = dest_dir.join("upstream");
    std::fs::create_dir_all(&upstream)?;

    for name in ["design.md", "tasks.md", "pipeline.toml"] {
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
