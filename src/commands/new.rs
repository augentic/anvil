//! `specify new` -- scaffold a new change directory.

use anyhow::Result;
use console::style;

use crate::core::change::Change;
use crate::core::config::ProjectConfig;
use crate::core::graph::ArtifactGraph;
use crate::core::paths::ProjectDir;
use crate::core::schema::Schema;

/// Run the `new` command.
///
/// # Errors
///
/// Returns an error if the project is not initialised, the change name is
/// invalid, or filesystem operations fail.
pub fn run(name: &str, json: bool) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let project = ProjectDir::discover(&cwd)?;
    let config = ProjectConfig::load(&project.config_file())?;
    let schema = Schema::load(&project, &config.schema)?;

    let change = Change::create(&project, name, &config.schema)?;

    if json {
        let out = serde_json::json!({
            "name": change.name,
            "path": change.path,
            "schema": change.metadata.schema,
            "created_at": change.metadata.created_at,
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }

    let graph = ArtifactGraph::from_schema(&schema)?;
    let order = graph.build_order();

    println!("\n  {} Created change {}\n", style("✓").green().bold(), style(name).cyan());
    println!("  Path:   {}", change.path.display());
    println!("  Schema: {} (v{})\n", config.schema, schema.version);
    println!("  Artifact order:");
    for (i, id) in order.iter().enumerate() {
        println!("    {}. {id}", i + 1);
    }
    println!();

    Ok(())
}
