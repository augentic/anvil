//! `specify archive` -- merge delta specs into baseline and archive the change.

use std::path::Path;

use anyhow::{Context, Result, bail};
use console::style;

use crate::core::change::Change;
use crate::core::config::ProjectConfig;
use crate::core::delta;
use crate::core::graph::ArtifactGraph;
use crate::core::paths::ProjectDir;
use crate::core::schema::Schema;

/// Run the `archive` command.
///
/// # Errors
///
/// Returns an error if the change is incomplete, delta merging fails, or
/// filesystem operations fail.
pub fn run(change_name: &str, json: bool) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let project = ProjectDir::discover(&cwd)?;
    let config = ProjectConfig::load(&project.config_file())?;
    let schema = Schema::load(&project, &config.schema)?;
    let change = Change::resolve(&project, Some(change_name))?;

    validate_completeness(&change, &schema)?;

    let merged = merge_delta_specs(&change, &project)?;

    let archive_path = change.archive(&project)?;

    if json {
        let out = serde_json::json!({
            "archived_to": archive_path,
            "specs_merged": merged,
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }

    println!("\n  {} Archived {}\n", style("✓").green().bold(), style(change_name).cyan(),);
    if merged.is_empty() {
        println!("  No delta specs to merge.");
    } else {
        println!("  Merged specs:");
        for name in &merged {
            println!("    {} {}", style("·").green(), name);
        }
    }
    println!("\n  Archived to: {}\n", archive_path.display());

    Ok(())
}

/// Verify all schema artifacts exist in the change directory.
fn validate_completeness(change: &Change, schema: &Schema) -> Result<()> {
    let graph = ArtifactGraph::from_schema(schema)?;
    let state = graph.completion(&change.path);

    if !state.ready.is_empty() || !state.blocked.is_empty() {
        let missing: Vec<String> =
            state.ready.into_iter().chain(state.blocked.into_iter().map(|b| b.id)).collect();
        bail!("change '{}' is incomplete; missing artifacts: {}", change.name, missing.join(", "));
    }

    Ok(())
}

/// For each `specs/<capability>/spec.md` in the change, merge into the
/// project's baseline at `specify/specs/<capability>/spec.md`.
///
/// Returns the list of capability names that were merged.
fn merge_delta_specs(change: &Change, project: &ProjectDir) -> Result<Vec<String>> {
    let change_specs = change.path.join("specs");
    if !change_specs.is_dir() {
        return Ok(Vec::new());
    }

    let mut merged = Vec::new();

    for entry in std::fs::read_dir(&change_specs)
        .with_context(|| format!("reading {}", change_specs.display()))?
    {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }

        let capability = entry.file_name().to_string_lossy().to_string();
        let delta_spec = entry.path().join("spec.md");
        if !delta_spec.is_file() {
            continue;
        }

        merge_one_spec(&capability, &delta_spec, project)?;
        merged.push(capability);
    }

    merged.sort();
    Ok(merged)
}

/// Merge a single delta spec into the baseline.
fn merge_one_spec(capability: &str, delta_path: &Path, project: &ProjectDir) -> Result<()> {
    let delta_content = std::fs::read_to_string(delta_path)
        .with_context(|| format!("reading {}", delta_path.display()))?;

    let sections = delta::parse_sections(&delta_content)
        .with_context(|| format!("parsing delta spec for '{capability}'"))?;

    let baseline_dir = project.spec_dir(capability);
    let baseline_path = baseline_dir.join("spec.md");

    let result = if baseline_path.is_file() {
        let baseline = std::fs::read_to_string(&baseline_path)
            .with_context(|| format!("reading {}", baseline_path.display()))?;
        delta::apply_to_baseline(&baseline, &sections)
            .with_context(|| format!("merging delta into baseline for '{capability}'"))?
    } else {
        build_new_baseline(&sections)
    };

    std::fs::create_dir_all(&baseline_dir)
        .with_context(|| format!("creating {}", baseline_dir.display()))?;
    std::fs::write(&baseline_path, result)
        .with_context(|| format!("writing {}", baseline_path.display()))?;

    Ok(())
}

/// When no baseline exists, create one from the ADDED requirements in the delta.
fn build_new_baseline(sections: &[delta::DeltaSection]) -> String {
    let mut content = String::new();
    for section in sections {
        if let delta::DeltaSection::Added(blocks) = section {
            for block in blocks {
                content.push_str(&block.raw_content);
                if !content.ends_with('\n') {
                    content.push('\n');
                }
            }
        }
    }
    content
}
