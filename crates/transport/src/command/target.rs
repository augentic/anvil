//! Clap argument types for `emery target *`. Each `*Args` type mirrors
//! its command's workflow wire input.

use std::path::PathBuf;

use clap::Args;

/// Arguments for `target resolve`.
#[derive(Debug, Args)]
pub struct ResolveArgs {
    /// Target-adapter identifier — kebab name or `name@version`
    /// (e.g. `omnia`, `vectis`, `contracts@1.0.0`). The optional
    /// `@version` suffix is treated as an opaque identifier and
    /// is stripped for the manifest lookup.
    pub value: String,
    /// Project directory containing `.emery/` (defaults to the
    /// current directory).
    #[arg(long, default_value = ".")]
    pub project_dir: Option<PathBuf>,
}
