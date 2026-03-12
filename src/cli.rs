//! Command-line interface definitions.

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

/// Admin CLI for Augentic's spec-driven development workflow.
///
/// Manages `OpenSpec` schemas, templates, and project configuration for
/// spec-driven development with Augentic tooling.
#[derive(Debug, Parser)]
#[command(name = "specify", version, about, long_about = None)]
pub struct Specify {
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
    /// Resolves the chosen schema, copies it into `specify/schemas/`, and
    /// writes `specify/config.yaml`.
    Init {
        /// Schema to use (skips interactive prompt).
        #[arg(long)]
        schema: Option<String>,
    },

    /// Create a new change.
    ///
    /// Scaffolds a change directory under `specify/changes/<name>/` with
    /// metadata and an empty `specs/` subdirectory.
    New {
        /// Kebab-case name for the change (e.g. `add-dark-mode`).
        name: String,

        /// Output machine-readable JSON.
        #[arg(long)]
        json: bool,
    },

    /// Show artifact completion status for a change.
    ///
    /// Reports which artifacts are complete, ready, or blocked.
    /// Auto-selects the change when only one is active.
    Status {
        /// Change name (optional if only one active change exists).
        change: Option<String>,

        /// Output machine-readable JSON.
        #[arg(long)]
        json: bool,
    },

    /// Output enriched instructions for creating an artifact.
    ///
    /// The instruction includes the schema instruction, project context, rules,
    /// template content, and dependency guidance. Use `apply` as the artifact
    /// to get apply-phase instructions.
    Instructions {
        /// Artifact ID (e.g. `proposal`, `specs`, `design`, `tasks`, `apply`).
        artifact: String,

        /// Change name (optional if only one active change exists).
        change: Option<String>,

        /// Output machine-readable JSON.
        #[arg(long)]
        json: bool,
    },

    /// List active and archived changes.
    List {
        /// Output machine-readable JSON.
        #[arg(long)]
        json: bool,
    },

    /// Archive a completed change.
    ///
    /// Merges delta specs from the change into the project's baseline specs
    /// at `specify/specs/`, then moves the change directory to a dated
    /// archive folder.
    Archive {
        /// Name of the change to archive.
        change: String,

        /// Output machine-readable JSON.
        #[arg(long)]
        json: bool,
    },

    /// Fetch the latest schemas from GitHub.
    Update {
        /// Also update the current project's specify/schemas/.
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
