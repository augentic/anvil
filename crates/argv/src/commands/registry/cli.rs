//! Clap argument types for `specify registry *`.

use clap::Args;
use serde::Serialize;

/// Argv mirror of `registry validate`'s wire input (no fields).
#[derive(Clone, Copy, Debug, Args, Serialize)]
#[expect(
    clippy::empty_structs_with_brackets,
    reason = "serde serialises the braced struct as the wire `{}` object a braced Input deserialises from"
)]
pub struct ValidateArgs {}

/// Argv mirror of `registry add`'s wire input
/// (`workflow::registry::handlers::AddInput`).
#[derive(Debug, Args, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct AddArgs {
    /// Kebab-case project name. Must be unique within the registry.
    pub name: String,
    /// Clone target — `.`, a repo-relative path, `git@host:path`, or
    /// `http(s)://` / `ssh://` / `git+...` remote.
    #[arg(long)]
    pub url: String,
    /// Optional greenfield scaffold seed — the adapter
    /// written into a brand-new project's `project.yaml` when
    /// `workspace sync` clones an empty repo. Not read for plan-time
    /// topology.
    #[arg(long)]
    pub adapter: Option<String>,
    /// Optional greenfield seed; a project's authoritative
    /// description lives in its own `project.yaml`.
    #[arg(long)]
    pub description: Option<String>,
}

/// Argv mirror of `registry remove`'s wire input
/// (`workflow::registry::handlers::RemoveInput`).
#[derive(Debug, Args, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct RemoveArgs {
    /// Kebab-case project name to remove.
    pub name: String,
}
