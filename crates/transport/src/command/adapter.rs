//! Clap argument types for `emery adapter *`. Each `*Args` type
//! mirrors its command's workflow wire input.

use std::path::PathBuf;

use clap::Args;

/// Arguments for `adapter add`.
#[derive(Debug, Args)]
pub struct AddArgs {
    /// Local `.wasm` component to seed into the project component
    /// cache; relative paths anchor at `--project-dir`.
    #[arg(value_name = "PATH")]
    pub component: PathBuf,
    /// Project directory the cache is keyed by (defaults to the
    /// nearest ancestor carrying `.emery/project.yaml`, else the
    /// current directory); `.emery/` need not exist yet.
    #[arg(long)]
    pub project_dir: Option<PathBuf>,
}

/// Arguments for `adapter upgrade`.
#[derive(Debug, Args)]
pub struct UpgradeArgs {
    /// Bare adapter name to upgrade to the newest published version.
    #[arg(value_name = "NAME", required_unless_present = "all", conflicts_with = "all")]
    pub name: Option<String>,
    /// Upgrade every bare adapter binding in the project
    /// (`project.yaml` target plus `plan.yaml` sources).
    #[arg(long)]
    pub all: bool,
    /// Project directory the `--all` collection anchors at (defaults
    /// to the nearest ancestor carrying `.emery/project.yaml`).
    #[arg(long)]
    pub project_dir: Option<PathBuf>,
}
