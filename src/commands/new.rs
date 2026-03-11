//! `alc new` -- scaffold a new change directory from schema templates.

use anyhow::{Context, Result, bail};
use console::style;

use crate::core::config::ProjectConfig;
use crate::core::paths::ProjectDir;
use crate::core::schema::Schema;

/// Run the new command.
///
/// # Errors
///
/// Returns an error if the project has no openspec config, the schema is missing,
/// the change already exists, or filesystem operations fail.
pub fn run(name: &str) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let project = ProjectDir::discover(&cwd)?;
    let config = ProjectConfig::load(&project.config_file())?;

    let schema_dir = project.schema_dir(&config.schema);
    let schema_yaml = std::fs::read(schema_dir.join("schema.yaml")).with_context(|| {
        format!(
            "reading schema '{}'; does openspec/schemas/{}/ exist?",
            config.schema, config.schema
        )
    })?;
    let schema = Schema::from_yaml(&schema_yaml)?;

    let change_dir = project.change_dir(name);
    if change_dir.is_dir() {
        bail!("change '{}' already exists at {}", name, change_dir.display());
    }

    std::fs::create_dir_all(&change_dir)
        .with_context(|| format!("creating {}", change_dir.display()))?;

    let templates_dir = schema_dir.join("templates");
    let mut scaffolded = Vec::new();

    for artifact in &schema.artifacts {
        if artifact.generates.contains("**") {
            let specs_dir = change_dir.join("specs");
            std::fs::create_dir_all(&specs_dir)?;
            scaffolded.push("specs/ (directory)".to_string());
            continue;
        }

        let template_path = templates_dir.join(&artifact.template);
        let dest_path = change_dir.join(&artifact.generates);

        if template_path.is_file() {
            let content = std::fs::read_to_string(&template_path)?;
            std::fs::write(&dest_path, content)?;
        } else {
            std::fs::write(&dest_path, "")?;
        }
        scaffolded.push(artifact.generates.clone());
    }

    println!("\n  {} Created change '{}'\n", style("✓").green().bold(), style(name).cyan());
    println!("  Location: {}\n", change_dir.display());
    println!("  Files:");
    for file in &scaffolded {
        println!("    - {file}");
    }
    println!(
        "\n  Start by editing {}\n",
        style(format!("openspec/changes/{name}/proposal.md")).yellow()
    );

    Ok(())
}
