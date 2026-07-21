//! Clap argument types for `specify adapter *`. Each `*Args` type
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
    /// nearest ancestor carrying `.specify/project.yaml`, else the
    /// current directory); `.specify/` need not exist yet.
    #[arg(long)]
    pub project_dir: Option<PathBuf>,
}
