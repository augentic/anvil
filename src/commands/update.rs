//! `specify update` -- fetch latest schemas from GitHub.

use anyhow::Result;
use console::style;

use crate::core::paths::ProjectDir;
use crate::core::registry;

/// Run the update command.
///
/// # Errors
///
/// Returns an error if schemas cannot be fetched from GitHub or written to disk.
pub fn run(project: bool, repo: &str, git_ref: &str) -> Result<()> {
    println!("\n  Fetching schemas from {}/{} ...", style(repo).cyan(), style(git_ref).cyan());

    let updated = registry::fetch_from_github(repo, git_ref)?;

    if updated.is_empty() {
        println!("  {} No schemas found in the repository.\n", style("⚠").yellow().bold());
        return Ok(());
    }

    println!(
        "  {} Updated {} schema(s) in local store:\n",
        style("✓").green().bold(),
        updated.len()
    );
    for name in &updated {
        println!("    - {}", style(name).cyan());
    }

    if project {
        update_project_schemas(&updated, repo, git_ref)?;
    }

    println!();
    Ok(())
}

/// Copy updated schemas into the current project's `openspec/schemas/`.
fn update_project_schemas(schema_names: &[String], _repo: &str, _git_ref: &str) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let project = ProjectDir::discover(&cwd)?;

    println!("\n  Updating project schemas at {} ...", project.schemas_dir().display());

    for name in schema_names {
        let resolved = registry::resolve(name)?;
        let dest = project.schema_dir(name);
        resolved.copy_to(&dest)?;
        println!("    {} {}", style("✓").green(), style(name).cyan());
    }

    Ok(())
}
