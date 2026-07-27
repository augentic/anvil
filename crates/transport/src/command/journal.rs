//! Clap argument types for `emery journal *`. Each `*Args` type
//! mirrors its command's workflow wire input.

use clap::Args;

/// Arguments for `journal emit`.
#[derive(Debug, Args)]
pub struct EmitArgs {
    /// Dotted-kebab event id (e.g. `slice.build.started`).
    pub event: String,

    /// JSON object carrying the event's payload fields (e.g.
    /// `{"source":"runtime","adapter":"captures",...}`).
    /// Omit for events with no payload fields.
    #[arg(long)]
    pub payload: Option<String>,
}

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
}
