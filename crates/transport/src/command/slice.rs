//! Clap argument types for `emery slice *`. Each `*Args` type mirrors
//! its command's workflow wire input.

use super::change_dir::ChangeDir;

/// Arguments for `slice list`.
#[derive(Clone, Debug, clap::Args)]
pub struct ListArgs {
    #[command(flatten)]
    pub change_dir: ChangeDir,
}

/// Arguments for `slice validate`.
#[derive(Debug, clap::Args)]
pub struct ValidateArgs {
    /// Slice name (under `.emery/change/slices/`)
    pub name: String,
    #[command(flatten)]
    pub change_dir: ChangeDir,
}

/// Arguments for `slice provenance`.
#[derive(Debug, clap::Args)]
pub struct ProvenanceArgs {
    /// Slice name (under `.emery/change/slices/`)
    pub name: String,
    #[command(flatten)]
    pub change_dir: ChangeDir,
}

/// Arguments for `slice model show`.
#[derive(Debug, clap::Args)]
pub struct ModelShowArgs {
    /// Slice name (under `.emery/change/slices/`)
    pub name: String,
    #[command(flatten)]
    pub change_dir: ChangeDir,
}
