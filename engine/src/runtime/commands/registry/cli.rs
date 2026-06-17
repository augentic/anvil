//! Clap derive surface for `specify registry *`. The umbrella
//! `cli.rs` re-exports `RegistryAction`.

use clap::Subcommand;

/// Registry operations on `registry.yaml`.
#[derive(Subcommand)]
pub enum RegistryAction {
    /// Validate `registry.yaml` shape. Absent file exits 0.
    Validate,
    /// Append a new project entry to `registry.yaml`. Creates the file
    /// when absent.
    Add {
        /// Kebab-case project name. Must be unique within the registry.
        name: String,
        /// Clone target — `.`, a repo-relative path, `git@host:path`, or
        /// `http(s)://` / `ssh://` / `git+...` remote.
        #[arg(long)]
        url: String,
        /// Optional greenfield scaffold seed — the adapter
        /// written into a brand-new project's `project.yaml` when
        /// `workspace sync` clones an empty repo. Not read for plan-time
        /// topology.
        #[arg(long)]
        adapter: Option<String>,
        /// Optional greenfield seed; a project's authoritative
        /// description lives in its own `project.yaml`.
        #[arg(long)]
        description: Option<String>,
    },
    /// Remove an existing project entry. Warns when `plan.yaml` references it.
    Remove {
        /// Kebab-case project name to remove.
        name: String,
    },
}
