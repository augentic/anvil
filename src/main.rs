mod agent;
mod apply;
mod archive;
mod brief;
mod cli;
mod context;
mod engine;
mod fan_out;
mod git;
mod github;
mod pipeline;
mod propose;
mod registry;
mod status;
mod sync;

use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::Parser;

use cli::{Cli, Command, RegistryAction};
use context::ChangeContext;
use engine::Engine;
use engine::opsx::OpsxEngine;
use registry::Registry;

/// Walk upward from the current directory to find the workspace root,
/// identified by the presence of `registry.toml`.
fn find_workspace_root() -> Result<PathBuf> {
    let mut dir = std::env::current_dir().context("cannot read current directory")?;
    loop {
        if dir.join("registry.toml").exists() {
            return Ok(dir);
        }
        if !dir.pop() {
            bail!("could not find registry.toml in any parent directory");
        }
    }
}

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

    let workspace = find_workspace_root()?;
    let concurrency = cli.concurrency;

    let engine: Box<dyn Engine> = match cli.engine.as_str() {
        "opsx" => Box::new(OpsxEngine),
        other => bail!("unknown engine '{other}' (supported: opsx)"),
    };

    let engine: &dyn Engine = &*engine;

    match cli.command {
        Command::Propose { change, description, dry_run } => {
            propose::run(&change, &description, dry_run, engine, &workspace).await?;
        }
        Command::FanOut { change, dry_run } => {
            fan_out::run(&change, dry_run, concurrency, engine, &workspace).await?;
        }
        Command::Apply { change, target, dry_run, continue_on_failure } => {
            apply::run(&change, target.as_deref(), dry_run, continue_on_failure, concurrency, engine, &workspace).await?;
        }
        Command::Status { change } => {
            let ctx = ChangeContext::load(&workspace, engine, &change)?;
            ctx.status.print_summary();
        }
        Command::Archive { change, dry_run } => {
            archive::run(&change, dry_run, engine, &workspace)?;
        }
        Command::Sync { change, mark_ready } => {
            sync::run(&change, mark_ready, engine, &workspace).await?;
        }
        Command::Registry { action } => {
            let reg = Registry::load(&workspace.join("registry.toml"))?;
            match action {
                RegistryAction::List => reg.print_all(),
                RegistryAction::Query { domain, cap } => {
                    if let Some(d) = &domain {
                        reg.print_by_domain(d);
                    }
                    if let Some(c) = &cap {
                        reg.print_by_capability(c);
                    }
                    if domain.is_none() && cap.is_none() {
                        bail!("provide --domain or --cap");
                    }
                }
            }
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
