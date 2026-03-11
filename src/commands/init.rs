//! `anvil init` -- initialise `OpenSpec` in the current project.

use anyhow::{Result, bail};
use console::style;
use dialoguer::{Input, Select};

use crate::core::config::ProjectConfig;
use crate::core::paths::ProjectDir;
use crate::core::registry;

/// Run the init command.
///
/// # Errors
///
/// Returns an error if the project cannot be initialised (directory exists without
/// `--force`, schema not found, or filesystem errors).
pub fn run(schema: Option<String>, context: Option<String>, force: bool) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let project = ProjectDir::from_root(&cwd);

    if project.exists() && !force {
        bail!(
            "openspec/ already exists at {}; use --force to reinitialise",
            project.root().display()
        );
    }

    let schema_name = resolve_schema_name(schema)?;
    let context_text = resolve_context(context)?;

    let resolved = registry::resolve(&schema_name)?;

    tracing::info!(schema = %schema_name, source = %resolved.source, "resolved schema");

    project.ensure()?;
    let dest = project.schema_dir(&schema_name);
    resolved.copy_to(&dest)?;

    let config = ProjectConfig::new(&schema_name, &context_text);
    config.write(&project.config_file())?;

    println!(
        "\n  {} Initialised OpenSpec in {}\n",
        style("✓").green().bold(),
        style(project.root().display()).cyan()
    );
    println!("  Schema:  {schema_name} (v{})", resolved.schema.version);
    println!("  Config:  {}", project.config_file().display());
    println!(
        "\n  Next steps:\n    1. Edit {} to customise rules",
        style("openspec/config.yaml").yellow()
    );
    println!("    2. Run {} to scaffold a change\n", style("anvil new <change-name>").yellow());

    Ok(())
}

/// Prompt for or validate the schema name.
fn resolve_schema_name(provided: Option<String>) -> Result<String> {
    if let Some(name) = provided {
        return Ok(name);
    }

    let available = registry::list_embedded();
    if available.is_empty() {
        bail!("no schemas available; run `anvil update` to fetch schemas from GitHub");
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

/// Prompt for or use the provided project context.
fn resolve_context(provided: Option<String>) -> Result<String> {
    if let Some(ctx) = provided {
        return Ok(ctx);
    }

    let ctx: String = Input::new()
        .with_prompt("Project context (tech stack, architecture)")
        .default("Rust project".to_string())
        .interact_text()?;

    Ok(ctx)
}
