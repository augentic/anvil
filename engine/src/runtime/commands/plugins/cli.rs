//! Clap derive surface for `specify plugins {doctor, refresh}`. The
//! umbrella `cli.rs` re-exports [`PluginsAction`].

use std::path::PathBuf;

use clap::Subcommand;

#[derive(Subcommand)]
pub enum PluginsAction {
    /// Report Cursor plugin-cache drift against the marketplace.
    ///
    /// Read-only. Resolves the marketplace (`--marketplace`, then
    /// `<project-dir>/.cursor-plugin/marketplace.json`, then the XDG
    /// config dir), scans `$CURSOR_HOME/plugins/cache/<name>/`, and
    /// classifies each declared plugin as
    /// `ok | drifted | present | missing`, plus any undeclared cache
    /// entry as `extra`. Never exits non-zero on drift — drift is a
    /// finding; only filesystem or marketplace-parse failures fail.
    Doctor {
        /// Project directory whose `.cursor-plugin/marketplace.json` is
        /// the second discovery candidate (defaults to the current
        /// directory). Bootstrap verb: no `.specify/` is required.
        #[arg(long, default_value = ".")]
        project_dir: PathBuf,
        /// Explicit marketplace file path; the first discovery
        /// candidate, ahead of the project and XDG locations.
        #[arg(long)]
        marketplace: Option<PathBuf>,
    },

    /// Invalidate the Cursor plugin cache for the marketplace.
    ///
    /// Deletes `$CURSOR_HOME/plugins/cache/<name>/`, journals
    /// `plugins.refreshed`, and prints a restart instruction. The CLI
    /// never restarts Cursor or touches open IDE state. Requires
    /// `--yes`; the verb never prompts (consent is the skill's job).
    Refresh {
        /// Project directory whose `.cursor-plugin/marketplace.json` is
        /// the second discovery candidate (defaults to the current
        /// directory).
        #[arg(long, default_value = ".")]
        project_dir: PathBuf,
        /// Explicit marketplace file path; the first discovery
        /// candidate.
        #[arg(long)]
        marketplace: Option<PathBuf>,
        /// Apply the cache deletion. Required to write; the verb never
        /// prompts interactively.
        #[arg(long)]
        yes: bool,
    },
}
