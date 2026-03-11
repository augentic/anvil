//! `specify schemas` -- list available schemas.

use anyhow::Result;
use console::style;

use crate::core::paths::ProjectDir;
use crate::core::registry::{self, SchemaEntry};

/// Run the schemas command.
///
/// # Errors
///
/// Returns an error if the local schema store cannot be read.
pub fn run() -> Result<()> {
    let cwd = std::env::current_dir()?;

    let embedded = registry::list_embedded();
    let local = registry::list_local_store().unwrap_or_default();
    let project = ProjectDir::discover(&cwd)
        .ok()
        .and_then(|p| registry::list_project(&p.schemas_dir()).ok())
        .unwrap_or_default();

    let active_schema = ProjectDir::discover(&cwd).ok().and_then(|p| {
        crate::core::config::ProjectConfig::load(&p.config_file()).ok().map(|c| c.schema)
    });

    let has_any = !embedded.is_empty() || !local.is_empty() || !project.is_empty();
    if !has_any {
        println!(
            "\n  No schemas found. Run {} to fetch schemas.\n",
            style("specify update").yellow()
        );
        return Ok(());
    }

    println!();
    print_section("Embedded", &embedded, active_schema.as_deref());
    print_section("Local store", &local, active_schema.as_deref());
    print_section("Project", &project, active_schema.as_deref());
    println!();

    Ok(())
}

fn print_section(title: &str, entries: &[SchemaEntry], active: Option<&str>) {
    if entries.is_empty() {
        return;
    }

    println!("  {} schemas:", style(title).bold());
    for entry in entries {
        let active_label = if active.is_some_and(|a| a == entry.name) { " (active)" } else { "" };

        println!(
            "    {} v{} -- {}{}",
            style(&entry.name).cyan(),
            entry.schema.version,
            entry.schema.description,
            style(active_label).green(),
        );
    }
    println!();
}
