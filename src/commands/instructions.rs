//! `specify instructions` -- output enriched instructions for creating an artifact.

use std::path::Path;

use anyhow::{Result, bail};
use serde::Serialize;

use crate::core::change::Change;
use crate::core::config::ProjectConfig;
use crate::core::graph::ArtifactGraph;
use crate::core::paths::ProjectDir;
use crate::core::schema::Schema;

/// Run the `instructions` command.
///
/// # Errors
///
/// Returns an error if the project is not initialised, the artifact ID is
/// unknown, or the schema/template cannot be loaded.
pub fn run(artifact_id: &str, change_name: Option<&str>, json: bool) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let project = ProjectDir::discover(&cwd)?;
    let config = ProjectConfig::load(&project.config_file())?;
    let schema = Schema::load(&project, &config.schema)?;
    let change = Change::resolve(&project, change_name)?;
    let graph = ArtifactGraph::from_schema(&schema)?;

    let enriched = if artifact_id == "apply" {
        build_apply_instructions(&schema, &config, &change.path, &graph)?
    } else {
        build_artifact_instructions(artifact_id, &schema, &config, &project, &change, &graph)?
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&enriched)?);
    } else {
        print_instructions(&enriched);
    }

    Ok(())
}

#[derive(Debug, Serialize)]
struct EnrichedInstructions {
    artifact: String,
    change: String,
    output_path: String,
    instruction: String,
    template: Option<String>,
    context: String,
    rules: Vec<String>,
    dependencies: Vec<String>,
}

fn build_artifact_instructions(
    artifact_id: &str, schema: &Schema, config: &ProjectConfig, project: &ProjectDir,
    change: &Change, graph: &ArtifactGraph,
) -> Result<EnrichedInstructions> {
    let artifact = schema
        .artifact(artifact_id)
        .ok_or_else(|| anyhow::anyhow!("unknown artifact '{artifact_id}'"))?;

    let template = load_template(project, &config.schema, &artifact.template);
    let rules = config.rules.get(artifact_id).cloned().unwrap_or_default();

    let deps = graph.dependencies_of(artifact_id);
    let dependency_context: Vec<String> = deps
        .iter()
        .map(|dep_id| {
            let generates = graph.generates(dep_id).unwrap_or("?");
            format!("Read {generates} (artifact: {dep_id}) before writing this artifact.")
        })
        .collect();

    let output_path = change.path.join(&artifact.generates);

    Ok(EnrichedInstructions {
        artifact: artifact_id.to_string(),
        change: change.name.clone(),
        output_path: output_path.to_string_lossy().to_string(),
        instruction: artifact.instruction.clone(),
        template,
        context: config.context.clone(),
        rules,
        dependencies: dependency_context,
    })
}

fn build_apply_instructions(
    schema: &Schema, config: &ProjectConfig, change_dir: &Path, graph: &ArtifactGraph,
) -> Result<EnrichedInstructions> {
    let apply =
        schema.apply.as_ref().ok_or_else(|| anyhow::anyhow!("schema has no apply phase"))?;

    let state = graph.completion(change_dir);
    if !state.apply_ready {
        bail!("apply phase is blocked; complete all required artifacts first");
    }

    let rules = config.rules.get("apply").cloned().unwrap_or_default();
    let change_name =
        change_dir.file_name().map_or_else(|| "?".to_string(), |n| n.to_string_lossy().to_string());

    Ok(EnrichedInstructions {
        artifact: "apply".to_string(),
        change: change_name,
        output_path: change_dir.to_string_lossy().to_string(),
        instruction: apply.instruction.clone(),
        template: None,
        context: config.context.clone(),
        rules,
        dependencies: vec!["All artifacts are complete. Implement the tasks.".to_string()],
    })
}

fn load_template(project: &ProjectDir, schema_name: &str, template_file: &str) -> Option<String> {
    let path = project.schema_dir(schema_name).join("templates").join(template_file);
    std::fs::read_to_string(path).ok()
}

fn print_instructions(instr: &EnrichedInstructions) {
    println!("\n--- {} (change: {}) ---\n", instr.artifact, instr.change);

    if !instr.context.is_empty() {
        println!("## Project Context\n\n{}\n", instr.context);
    }

    if !instr.dependencies.is_empty() {
        println!("## Dependencies\n");
        for dep in &instr.dependencies {
            println!("- {dep}");
        }
        println!();
    }

    if !instr.rules.is_empty() {
        println!("## Rules\n");
        for rule in &instr.rules {
            println!("- {rule}");
        }
        println!();
    }

    println!("## Instruction\n\n{}", instr.instruction);

    if let Some(template) = &instr.template {
        println!("## Template\n\n{template}");
    }

    println!("## Output\n\nWrite to: {}\n", instr.output_path);
}
