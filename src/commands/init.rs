//! `specify init` -- install `OpenSpec` and initialise it in the current project.

use std::process::Command;

use anyhow::{Context, Result, bail};
use console::style;
use dialoguer::{Input, Select};

use crate::core::config::ProjectConfig;
use crate::core::paths::ProjectDir;
use crate::core::registry;

/// Run the init command.
///
/// Ensures the `openspec` CLI is installed (via Homebrew), then resolves the
/// chosen schema, copies it into `openspec/schemas/`, and writes
/// `openspec/config.yaml`.
///
/// # Errors
///
/// Returns an error if Homebrew is unavailable, openspec installation fails,
/// or the specify-specific layering encounters errors.
pub fn run(schema: Option<String>, context: Option<String>) -> Result<()> {
    ensure_openspec_installed()?;

    let cwd = std::env::current_dir()?;
    let project = ProjectDir::from_root(&cwd);

    let schema_name = resolve_schema_name(schema)?;
    let context_text = resolve_context(context)?;

    let resolved = registry::resolve(&schema_name)?;

    tracing::info!(schema = %schema_name, source = %resolved.source, "resolved schema");

    project.ensure()?;
    let dest = project.schema_dir(&schema_name);
    resolved.copy_to(&dest)?;

    let config = ProjectConfig::new(&schema_name, &context_text);
    config.write(&project.config_file())?;

    println!("\n  {} Specify configuration written\n", style("✓").green().bold(),);
    println!("  Schema:  {schema_name} (v{})", resolved.schema.version);
    println!("  Config:  {}", project.config_file().display());
    println!(
        "\n  Next steps:\n    1. Edit {} to customise rules",
        style("openspec/config.yaml").yellow()
    );
    println!("    2. Run {} to start a change\n", style("/opsx:propose <description>").yellow());

    Ok(())
}

/// Check whether `openspec` is on PATH; if not, install it via Homebrew.
fn ensure_openspec_installed() -> Result<()> {
    if is_openspec_available() {
        tracing::debug!("openspec already installed");
        return Ok(());
    }

    println!(
        "\n  {} openspec CLI not found -- installing via Homebrew...\n",
        style("→").cyan().bold(),
    );

    if !is_brew_available() {
        bail!(
            "Homebrew is required to install openspec but `brew` was not found.\n  \
             Install Homebrew from https://brew.sh then re-run `specify init`."
        );
    }

    let status = Command::new("brew")
        .args(["install", "openspec"])
        .status()
        .context("failed to run `brew install openspec`")?;

    if !status.success() {
        bail!("`brew install openspec` failed (exit code: {status})");
    }

    if !is_openspec_available() {
        bail!(
            "openspec was installed but is not available on PATH; check your shell configuration"
        );
    }

    Ok(())
}

fn is_openspec_available() -> bool {
    Command::new("openspec").arg("--version").output().is_ok_and(|o| o.status.success())
}

fn is_brew_available() -> bool {
    Command::new("brew").arg("--version").output().is_ok_and(|o| o.status.success())
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
