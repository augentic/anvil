//! Command-line interface definitions.

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

/// Admin CLI for Augentic's spec-driven development workflow.
///
/// Manages `OpenSpec` schemas, templates, and project configuration for
/// spec-driven development with Augentic tooling.
#[derive(Debug, Parser)]
#[command(name = "anvil", version, about, long_about = None)]
pub struct Anvil {
    /// Increase log verbosity (-v for debug, -vv for trace).
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Suppress non-error output.
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Subcommand to execute.
    #[command(subcommand)]
    pub command: Command,
}

/// Top-level subcommands.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Initialise `OpenSpec` in the current project.
    ///
    /// Installs the `openspec` CLI via Homebrew if needed, runs
    /// `openspec init --tools cursor --force`, then layers on
    /// anvil-specific schema and configuration.
    Init {
        /// Schema to use (skips interactive prompt).
        #[arg(long)]
        schema: Option<String>,

        /// Project context description (skips interactive prompt).
        #[arg(long)]
        context: Option<String>,
    },

    /// Fetch the latest schemas from GitHub.
    Update {
        /// Also update the current project's openspec/schemas/.
        #[arg(long)]
        project: bool,

        /// GitHub repository to fetch schemas from.
        #[arg(long, default_value = "augentic/lifecycle")]
        repo: String,

        /// Git ref (branch or tag) to fetch from.
        #[arg(long, default_value = "main")]
        git_ref: String,
    },

    /// Validate project `OpenSpec` configuration and structure.
    Validate,

    /// List available schemas.
    Schemas,

    /// Generate shell completions.
    Completions {
        /// Shell to generate completions for.
        shell: ShellChoice,

        /// Directory to write completions to (defaults to stdout).
        #[arg(long)]
        output: Option<PathBuf>,
    },
}

/// Supported shells for completion generation.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ShellChoice {
    /// Bash shell.
    Bash,
    /// Zsh shell.
    Zsh,
    /// Fish shell.
    Fish,
    /// `PowerShell`.
    PowerShell,
}

impl From<ShellChoice> for clap_complete::Shell {
    fn from(s: ShellChoice) -> Self {
        match s {
            ShellChoice::Bash => Self::Bash,
            ShellChoice::Zsh => Self::Zsh,
            ShellChoice::Fish => Self::Fish,
            ShellChoice::PowerShell => Self::PowerShell,
        }
    }
}
