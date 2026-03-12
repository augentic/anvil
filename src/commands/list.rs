//! `specify list` -- list active and archived changes.

use anyhow::Result;
use console::style;
use serde::Serialize;

use crate::core::change::Change;
use crate::core::config::ProjectConfig;
use crate::core::graph::ArtifactGraph;
use crate::core::paths::ProjectDir;
use crate::core::schema::Schema;

/// Run the `list` command.
///
/// # Errors
///
/// Returns an error if the project is not initialised or changes cannot be
/// discovered.
pub fn run(json: bool) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let project = ProjectDir::discover(&cwd)?;
    let config = ProjectConfig::load(&project.config_file())?;
    let schema = Schema::load(&project, &config.schema).ok();

    let active = Change::discover_active(&project)?;
    let archived = Change::discover_archived(&project)?;

    if json {
        let entries: Vec<ListEntry> = active
            .iter()
            .map(|c| to_entry(c, false, schema.as_ref()))
            .chain(archived.iter().map(|c| to_entry(c, true, schema.as_ref())))
            .collect();
        println!("{}", serde_json::to_string_pretty(&entries)?);
        return Ok(());
    }

    if active.is_empty() && archived.is_empty() {
        println!("\n  No changes found. Run {} to start.\n", style("specify new <name>").yellow(),);
        return Ok(());
    }

    if !active.is_empty() {
        println!("\n  {} changes:\n", style("Active").bold());
        for change in &active {
            print_change(change, schema.as_ref());
        }
    }

    if !archived.is_empty() {
        println!("\n  {} changes:\n", style("Archived").bold());
        for change in &archived {
            println!(
                "    {} {} {}",
                style("·").dim(),
                style(&change.name).dim(),
                style(&change.metadata.created_at).dim(),
            );
        }
    }

    println!();
    Ok(())
}

fn print_change(change: &Change, schema: Option<&Schema>) {
    let summary = completion_summary(change, schema);
    println!(
        "    {} {} {} {}",
        style("·").cyan(),
        style(&change.name).cyan(),
        style(&change.metadata.created_at).dim(),
        summary,
    );
}

fn completion_summary(change: &Change, schema: Option<&Schema>) -> String {
    let Some(schema) = schema else {
        return String::new();
    };
    let Ok(graph) = ArtifactGraph::from_schema(schema) else {
        return String::new();
    };
    let state = graph.completion(&change.path);
    let total = graph.artifact_ids().len();
    let done = state.completed.len();
    format!("[{done}/{total}]")
}

#[derive(Debug, Serialize)]
struct ListEntry {
    name: String,
    schema: String,
    created_at: String,
    archived: bool,
    artifacts_completed: usize,
    artifacts_total: usize,
}

fn to_entry(change: &Change, archived: bool, schema: Option<&Schema>) -> ListEntry {
    let (completed, total) =
        schema.and_then(|s| ArtifactGraph::from_schema(s).ok()).map_or((0, 0), |graph| {
            let state = graph.completion(&change.path);
            (state.completed.len(), graph.artifact_ids().len())
        });

    ListEntry {
        name: change.name.clone(),
        schema: change.metadata.schema.clone(),
        created_at: change.metadata.created_at.clone(),
        archived,
        artifacts_completed: completed,
        artifacts_total: total,
    }
}
