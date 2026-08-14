//! Clap argument types for `emery journal *`. Each `*Args` type
//! mirrors its command's workflow wire input.

use clap::Args;

use super::change_dir::ChangeDir;

/// Arguments for `journal show`.
#[derive(Debug, Args)]
pub struct ShowArgs {
    /// Keep only events whose dotted-kebab id starts with this
    /// prefix (e.g. `slice.build` or `plan.entry.advanced`).
    #[arg(long)]
    pub filter: Option<String>,

    /// Keep only the most recent N matching events.
    #[arg(long)]
    pub limit: Option<usize>,
    #[command(flatten)]
    pub change_dir: ChangeDir,
}
