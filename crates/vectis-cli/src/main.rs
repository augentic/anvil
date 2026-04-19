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
#[derive(Debug)]
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
                  verifies that every assembly compiles. See RFC-6."
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

// Fields below are populated by clap. `InitArgs` is fully consumed by
// chunk 5 (`app_name`, `dir`, `version_file`, `android_package` -- the
// last as the source of the default Android package even for core-only
// scaffolds, since `__ANDROID_PACKAGE__` lives in `codegen.rs`) and
// chunk 6 (`caps`). See rfcs/rfc-6-tasks.md.
#[derive(clap::Args, Debug)]
pub(crate) struct InitArgs {
    /// App struct name (PascalCase, e.g. "Counter", "TodoApp").
    pub app_name: String,

    /// Project directory (defaults to current directory).
    #[arg(long)]
    pub dir: Option<std::path::PathBuf>,

    /// Comma-separated capabilities. Values: http, kv, time, platform, sse.
    #[arg(long)]
    pub caps: Option<String>,

    /// Comma-separated shell platforms. Values: ios, android.
    #[arg(long)]
    pub shells: Option<String>,

    /// Android package name (defaults to com.vectis.<appname lowercase>).
    #[arg(long)]
    pub android_package: Option<String>,

    /// Override version pins file. When set, the file MUST exist; resolution
    /// otherwise falls back to `<project>/versions.toml`,
    /// `~/.config/vectis/versions.toml`, then the embedded defaults.
    #[arg(long)]
    pub version_file: Option<std::path::PathBuf>,
}

#[derive(clap::Args, Debug)]
pub(crate) struct VerifyArgs {
    /// Project directory (defaults to current directory).
    #[arg(long)]
    pub dir: Option<std::path::PathBuf>,

    /// Override version pins file. When set, the file MUST exist; see
    /// `vectis init --help` for the full resolution order.
    #[arg(long)]
    pub version_file: Option<std::path::PathBuf>,
}

#[derive(clap::Args, Debug)]
pub(crate) struct AddShellArgs {
    /// Shell platform to add. Values: ios, android.
    pub platform: String,

    /// Project directory (defaults to current directory).
    #[arg(long)]
    pub dir: Option<std::path::PathBuf>,

    /// Android package name (defaults to com.vectis.<appname lowercase>).
    #[arg(long)]
    pub android_package: Option<String>,

    /// Override version pins file. When set, the file MUST exist; see
    /// `vectis init --help` for the full resolution order.
    #[arg(long)]
    pub version_file: Option<std::path::PathBuf>,
}

#[derive(clap::Args, Debug)]
pub(crate) struct UpdateVersionsArgs {
    /// File to update (defaults to ~/.config/vectis/versions.toml). For
    /// `update-versions` this is the *write target*, not a resolution
    /// override -- on the other subcommands the same flag overrides
    /// resolution.
    #[arg(long)]
    pub version_file: Option<std::path::PathBuf>,

    /// Show proposed changes without writing.
    #[arg(long)]
    pub dry_run: bool,

    /// Scaffold a scratch project and run `vectis verify` before committing pins.
    #[arg(long)]
    pub verify: bool,
}

fn main() {
    let cli = Cli::parse();

    let result: Result<CommandOutcome, VectisError> = match &cli.command {
        Command::Init(args) => init::run(args),
        Command::Verify(args) => verify::run(args),
        Command::AddShell(args) => add_shell::run(args),
        Command::UpdateVersions(args) => update_versions::run(args),
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
