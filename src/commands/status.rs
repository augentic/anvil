//! `specify status` -- report artifact completion state for a change.

use anyhow::Result;
use console::style;

use crate::core::change::Change;
use crate::core::config::ProjectConfig;
use crate::core::graph::{ArtifactGraph, CompletionState};
use crate::core::paths::ProjectDir;
use crate::core::schema::Schema;

/// Run the `status` command.
///
/// # Errors
///
/// Returns an error if the project is not initialised, the change cannot be
/// resolved, or the schema cannot be loaded.
pub fn run(change_name: Option<&str>, json: bool) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let project = ProjectDir::discover(&cwd)?;
    let config = ProjectConfig::load(&project.config_file())?;
    let schema = Schema::load(&project, &config.schema)?;
    let change = Change::resolve(&project, change_name)?;
    let graph = ArtifactGraph::from_schema(&schema)?;
    let state = graph.completion(&change.path);

    if json {
        println!("{}", serde_json::to_string_pretty(&state)?);
        return Ok(());
    }

    print_status(&change.name, &graph, &schema, &state);
    Ok(())
}

fn print_status(name: &str, graph: &ArtifactGraph, schema: &Schema, state: &CompletionState) {
    println!("\n  {}\n", style(name).bold());

    for id in graph.artifact_ids() {
        let generates = graph.generates(id).unwrap_or("?");

        if state.completed.contains(id) {
            let progress = task_progress_label(id, state);
            println!("  {} {}  {}{}", style("[x]").green(), style(id).green(), generates, progress,);
        } else if state.ready.iter().any(|r| r == id) {
            println!(
                "  {} {}  {} {}",
                style("[ ]").white(),
                style(id).yellow(),
                generates,
                style("(ready)").dim(),
            );
        } else if let Some(blocked) = state.blocked.iter().find(|b| b.id == *id) {
            println!(
                "  {} {}  {} {}",
                style("[ ]").white().dim(),
                style(id).dim(),
                generates,
                style(format!("(blocked: {})", blocked.waiting_on.join(", "))).dim(),
            );
        }
    }

    println!();
    if state.apply_ready {
        if let Some(progress) = &state.task_progress {
            println!(
                "  apply: {} ({}/{} tasks)",
                style("ready").green(),
                progress.done,
                progress.total,
            );
        } else {
            println!("  apply: {}", style("ready").green());
        }
    } else {
        let missing: Vec<&str> = schema
            .apply
            .as_ref()
            .map_or(&[] as &[String], |a| a.requires.as_slice())
            .iter()
            .filter(|r| !state.completed.contains(r))
            .map(String::as_str)
            .collect();
        println!("  apply: {} (requires: {})", style("blocked").dim(), missing.join(", "),);
    }
    println!();
}

fn task_progress_label(id: &str, state: &CompletionState) -> String {
    if id != "tasks" {
        return String::new();
    }
    state.task_progress.map_or_else(String::new, |p| format!(" ({}/{})", p.done, p.total))
}
