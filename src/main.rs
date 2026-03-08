use anyhow::{Result, bail};
use clap::Parser;

use alc::cli::{Cli, Command, RegistryAction};
use alc::context::ChangeContext;
use alc::engine::opsx::OpsxEngine;
use alc::registry::Registry;
use alc::session::Session;
use alc::{apply, archive, output, propose, workspace};

async fn run() -> Result<()> {
    let cli = Cli::parse();

    let default_level = if cli.verbose {
        "debug"
    } else if cli.quiet {
        "warn"
    } else {
        "info"
    };

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(default_level)),
        )
        .without_time()
        .init();

    if matches!(&cli.command, Command::Init) {
        return init_workspace();
    }

    let workspace = workspace::find_root()?;
    let session = Session::new(workspace, cli.concurrency);

    match cli.command {
        Command::Init => unreachable!(),
        Command::Propose { change, description, dry_run } => {
            propose::run(&change, &description, dry_run, &session).await?;
        }
        Command::Apply { change, target, dry_run, continue_on_failure } => {
            apply::run(&change, target.as_deref(), dry_run, continue_on_failure, &session).await?;
        }
        Command::Status { change } => {
            let ctx = ChangeContext::load(&session, &change)?;
            output::print_status_summary(&ctx.status);
        }
        Command::Validate { change } => {
            let _ctx = ChangeContext::load(&session, &change)?;
            println!("change '{change}' is valid: pipeline, registry, and status are consistent");
        }
        Command::List => {
            let changes_dir = session.workspace.join(session.engine.changes_dir());
            list_changes(&changes_dir)?;
        }
        Command::Archive { change, mark_ready, auto_merge } => {
            archive::run(&change, mark_ready, auto_merge, &session).await?;
        }
        Command::Registry { action } => {
            let reg = Registry::load(&session.workspace.join("registry.toml"))?;
            match action {
                RegistryAction::List => output::print_registry(&reg),
                RegistryAction::Query { domain, cap } => {
                    if domain.is_none() && cap.is_none() {
                        bail!("provide --domain or --cap");
                    }
                    if let Some(d) = &domain {
                        output::print_services_by_domain(&reg.find_by_domain(d), d);
                    }
                    if let Some(c) = &cap {
                        output::print_services_by_capability(&reg.find_by_capability(c), c);
                    }
                }
            }
        }
    }

    Ok(())
}

fn init_workspace() -> Result<()> {
    use anyhow::Context;
    use std::path::Path;

    let registry_path = Path::new("registry.toml");
    if registry_path.exists() {
        bail!("registry.toml already exists in the current directory");
    }

    let engine = OpsxEngine;
    let template = r#"# Service registry — add one [[services]] entry per target service.
# See: https://github.com/augentic/lifecycle#registrytoml

# [[services]]
# id = "my-service"
# repo = "git@github.com:org/my-service.git"
# project_dir = "."
# crate = "my-service"
# domain = "core"
# capabilities = ["ingest"]
"#;
    std::fs::write(registry_path, template).context("writing registry.toml")?;
    std::fs::create_dir_all(engine.changes_dir()).context("creating changes directory")?;
    std::fs::create_dir_all(engine.specs_dir()).context("creating specs directory")?;
    std::fs::create_dir_all(engine.domains_dir()).context("creating domains directory")?;

    println!("initialised alc workspace:");
    println!("  registry.toml        — add your services here");
    println!("  {}/    — change artefacts", engine.changes_dir());
    println!("  {}/      — shared specs", engine.specs_dir());
    println!("  {}/          — domain context docs", engine.domains_dir());
    Ok(())
}

fn list_changes(changes_dir: &std::path::Path) -> Result<()> {
    use anyhow::Context;

    if !changes_dir.is_dir() {
        bail!("no changes directory at {}", changes_dir.display());
    }

    let mut changes: Vec<String> = Vec::new();
    let entries = std::fs::read_dir(changes_dir)
        .with_context(|| format!("reading {}", changes_dir.display()))?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir()
            && let Some(name) = path.file_name().and_then(|n| n.to_str())
            && name != "archive"
        {
            changes.push(name.to_string());
        }
    }

    changes.sort();

    if changes.is_empty() {
        println!("no changes found in {}", changes_dir.display());
    } else {
        println!("changes:");
        for name in &changes {
            let has_pipeline = changes_dir.join(name).join("pipeline.toml").exists();
            let has_status = changes_dir.join(name).join("status.toml").exists();
            let indicator = match (has_pipeline, has_status) {
                (true, true) => "pipeline + status",
                (true, false) => "pipeline",
                _ => "scaffold only",
            };
            println!("  {name:<30} ({indicator})");
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}
