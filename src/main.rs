mod agent;
mod apply;
mod archive;
mod brief;
mod cli;
mod fan_out;
mod git;
mod pipeline;
mod propose;
mod registry;
mod engine;
mod status;
mod sync;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::Parser;

use cli::{Cli, Command, RegistryAction};
use registry::Registry;
use engine::opsx::OpsxEngine;

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

/// Resolve the change directory within the workspace.
fn change_dir(workspace: &Path, engine: &dyn engine::Engine, change: &str) -> PathBuf {
    workspace.join(engine.changes_dir()).join(change)
}

fn run() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .without_time()
        .init();

    let cli = Cli::parse();
    let workspace = find_workspace_root()?;
    let engine = OpsxEngine;

    match cli.command {
        Command::Propose { change, description, dry_run } => {
            propose::run(&change, &description, dry_run, &engine, &workspace)?;
        }
        Command::FanOut { change, dry_run } => {
            fan_out::run(&change, dry_run, &engine, &workspace)?;
        }
        Command::Apply { change, target, dry_run } => {
            apply::run(&change, target.as_deref(), dry_run, &engine, &workspace)?;
        }
        Command::Status { change } => {
            let change_dir = change_dir(&workspace, &engine, &change);
            let pipeline = pipeline::Pipeline::load(&change_dir.join("pipeline.toml"))?;
            let registry = Registry::load(&workspace.join("registry.toml"))?;
            pipeline.validate(&registry, &change_dir)?;
            let status = status::PipelineStatus::load_or_create(
                &change_dir.join("status.toml"),
                &change,
                &pipeline,
                &registry,
            )?;
            status.print_summary();
        }
        Command::Archive { change, dry_run } => {
            archive::run(&change, dry_run, &engine, &workspace)?;
        }
        Command::Sync { change, mark_ready } => {
            sync::run(&change, mark_ready, &workspace)?;
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

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}
