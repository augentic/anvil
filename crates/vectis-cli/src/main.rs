//! `vectis` CLI entry point.
//!
//! Chunk 1 establishes the dispatch skeleton: every subcommand parses its
//! arguments, calls a handler, and either prints the handler's success JSON
//! (exit 0) or the structured error JSON (non-zero exit). The handlers
//! themselves are stubs that return `not_implemented`; later chunks fill them
//! in without changing this dispatch layer.

use clap::{Parser, Subcommand};

mod add_shell;
mod error;
mod init;
mod prerequisites;
mod templates;
mod update_versions;
mod verify;
mod versions;

use error::VectisError;

/// Outcome returned by every subcommand handler.
///
/// `Success` carries the handler's normal JSON output and exits 0. `Stub` is a
/// placeholder for handlers that have not been implemented yet (chunks 5+
/// replace these); it prints the RFC's `not_implemented` shape and exits 1.
/// Real failures flow through `Err(VectisError)`.
pub enum CommandOutcome {
    Success(serde_json::Value),
    Stub { command: &'static str },
}

#[derive(Parser, Debug)]
#[command(
    name = "vectis",
    version,
    about = "Bootstrap and verify Crux cross-platform projects",
    long_about = "Vectis CLI -- scaffolds the deterministic 'Hello World' starting \
                  point for Crux apps (core + optional iOS/Android shells) and \
                  verifies that every assembly compiles. See RFC-5."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Scaffold a new Crux project (core, plus optional shells).
    Init(InitArgs),

    /// Verify that the project's assemblies compile.
    Verify(VerifyArgs),

    /// Add a platform shell to an existing project.
    AddShell(AddShellArgs),

    /// Resolve and pin coherent dependency versions.
    UpdateVersions(UpdateVersionsArgs),
}

#[derive(clap::Args, Debug)]
struct InitArgs {
    /// App struct name (PascalCase, e.g. "Counter", "TodoApp").
    app_name: String,

    /// Project directory (defaults to current directory).
    #[arg(long)]
    dir: Option<std::path::PathBuf>,

    /// Comma-separated capabilities. Values: http, kv, time, platform, sse.
    #[arg(long)]
    caps: Option<String>,

    /// Comma-separated shell platforms. Values: ios, android.
    #[arg(long)]
    shells: Option<String>,

    /// Android package name (defaults to com.vectis.<appname lowercase>).
    #[arg(long)]
    android_package: Option<String>,
}

#[derive(clap::Args, Debug)]
struct VerifyArgs {
    /// Project directory (defaults to current directory).
    #[arg(long)]
    dir: Option<std::path::PathBuf>,
}

#[derive(clap::Args, Debug)]
struct AddShellArgs {
    /// Shell platform to add. Values: ios, android.
    platform: String,

    /// Project directory (defaults to current directory).
    #[arg(long)]
    dir: Option<std::path::PathBuf>,

    /// Android package name (defaults to com.vectis.<appname lowercase>).
    #[arg(long)]
    android_package: Option<String>,
}

#[derive(clap::Args, Debug)]
struct UpdateVersionsArgs {
    /// File to update (defaults to ~/.config/vectis/versions.toml).
    #[arg(long)]
    version_file: Option<std::path::PathBuf>,

    /// Show proposed changes without writing.
    #[arg(long)]
    dry_run: bool,

    /// Scaffold a scratch project and run `vectis verify` before committing pins.
    #[arg(long)]
    verify: bool,
}

fn main() {
    let cli = Cli::parse();

    let result: Result<CommandOutcome, VectisError> = match &cli.command {
        Command::Init(_) => init::run(),
        Command::Verify(_) => verify::run(),
        Command::AddShell(_) => add_shell::run(),
        Command::UpdateVersions(_) => update_versions::run(),
    };

    match result {
        Ok(CommandOutcome::Success(value)) => {
            println!("{}", serde_json::to_string_pretty(&value).unwrap());
        }
        Ok(CommandOutcome::Stub { command }) => {
            let value = serde_json::json!({
                "error": "not_implemented",
                "command": command,
            });
            println!("{}", serde_json::to_string_pretty(&value).unwrap());
            std::process::exit(1);
        }
        Err(err) => {
            println!("{}", serde_json::to_string_pretty(&err.to_json()).unwrap());
            std::process::exit(err.exit_code());
        }
    }
}
