//! Application-wide and provisioning clap argument types.

use std::path::PathBuf;

use clap::{Args, ValueEnum};

pub use crate::output::Format;

/// One-line application description.
pub const ABOUT: &str = "Deterministic primitives for spec-driven development";

/// Arguments shared by every command route.
#[derive(Clone, Debug, Args)]
pub struct Globals {
    /// Output format.
    #[arg(long, env = "SPECIFY_FORMAT", default_value = "text")]
    pub format: Format,
    /// Directory holding the governing plan.
    #[arg(long, env = "SPECIFY_PLAN_DIR", value_name = "PATH")]
    pub plan_dir: Option<PathBuf>,
}

/// Flags for `specify init`.
#[derive(Debug, Args)]
pub struct InitArgs {
    /// Adapter identifier or local component path.
    #[arg(conflicts_with = "workspace")]
    pub adapter: Option<String>,
    /// Project name.
    #[arg(long)]
    pub name: Option<String>,
    /// Project description.
    #[arg(long)]
    pub description: Option<String>,
    /// Scaffold a registry-only workspace.
    #[arg(long)]
    pub workspace: bool,
    /// Comma-separated target platforms.
    #[arg(long, conflicts_with = "workspace")]
    pub platforms: Option<String>,
    /// Re-enter initialization to update the Specify version pin.
    #[arg(long, conflicts_with_all = ["adapter", "workspace", "name", "description"])]
    pub upgrade: bool,
    /// Run only the guest-supported scaffold leg.
    #[arg(long, hide = true, conflicts_with = "upgrade")]
    pub scaffold_only: bool,
}

/// Flags for `specify upgrade`.
#[derive(Debug, Clone, Copy, Args)]
pub struct UpgradeArgs {
    /// Install channel to upgrade.
    #[arg(long, value_enum, default_value = "auto")]
    pub channel: ChannelArg,
    /// Apply the upgrade.
    #[arg(long)]
    pub yes: bool,
    /// Report the upgrade plan without changing anything.
    #[arg(long)]
    pub dry_run: bool,
}

/// `specify upgrade --channel` value.
#[derive(Copy, Clone, Debug, ValueEnum, PartialEq, Eq)]
pub enum ChannelArg {
    /// Detect the install channel.
    Auto,
    /// Force the Cargo strategy.
    Cargo,
    /// Force the Homebrew strategy.
    Brew,
    /// Force the release-archive strategy.
    Binary,
}
