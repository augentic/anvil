//! `specify init` -- initialise `OpenSpec` in the current project.

use anyhow::{Context, Result, bail};
use console::style;
use dialoguer::Select;

use crate::core::config::ProjectConfig;
use crate::core::paths::ProjectDir;
use crate::core::registry;

/// Run the init command.
///
/// Resolves the chosen schema, copies it into `specify/schemas/`, creates the
/// `specify/changes/` and `specify/specs/` directories, and writes
/// `specify/config.yaml`.
///
/// # Errors
///
/// Returns an error if schema resolution or filesystem operations fail.
pub fn run(schema: Option<String>) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let project = ProjectDir::from_root(&cwd);

    let schema_name = resolve_schema_name(schema)?;

    let resolved = registry::resolve(&schema_name)?;

    tracing::info!(schema = %schema_name, source = %resolved.source, "resolved schema");

    project.ensure()?;
    let dest = project.schema_dir(&schema_name);
    resolved.copy_to(&dest)?;

    let template_path = dest.join("config.yaml");
    if template_path.is_file() {
        std::fs::copy(&template_path, project.config_file())
            .with_context(|| format!("copying config template from {}", template_path.display()))?;
    } else {
        let config = ProjectConfig::new(&schema_name, "");
        config.write(&project.config_file())?;
    }

    println!("\n  {} Specify configuration written\n", style("✓").green().bold());
    println!("  Schema:  {schema_name} (v{})", resolved.schema.version);
    println!("  Config:  {}", project.config_file().display());
    println!(
        "\n  Next steps:\n    1. Edit {} to customise context and rules",
        style("specify/config.yaml").yellow()
    );
    println!("    2. Run {} to start a change\n", style("specify new <name>").yellow());

    Ok(())
}

/// Prompt for or validate the schema name.
fn resolve_schema_name(provided: Option<String>) -> Result<String> {
    if let Some(name) = provided {
        return Ok(name);
    }

    let available = registry::list_embedded();
    if available.is_empty() {
        bail!("no schemas available; run `specify update` to fetch schemas from GitHub");
    }

    if available.len() == 1 {
        let name = &available[0].name;
        println!("  Using schema: {} (only available schema)", style(name).cyan());
        return Ok(name.clone());
    }

    let names: Vec<&str> = available.iter().map(|e| e.name.as_str()).collect();
    let selection =
        Select::new().with_prompt("Select a schema").items(&names).default(0).interact()?;

    Ok(names[selection].to_string())
}
