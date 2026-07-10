//! Clap argument types for `specify journal *`.

use clap::Args;
use serde::Serialize;

/// Argv mirror of `journal emit`'s wire input
/// (`workflow::journal::handlers::EmitInput`).
#[derive(Debug, Args, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct EmitArgs {
    /// Dotted-kebab event id (e.g. `slice.build.started`).
    pub event: String,

    /// JSON object carrying the event's payload fields (e.g.
    /// `{"source":"runtime","adapter":"captures",...}`).
    /// Omit for events with no payload fields.
    #[arg(long)]
    pub payload: Option<String>,
}

/// Argv mirror of `journal show`'s wire input
/// (`workflow::journal::handlers::ShowInput`).
#[derive(Debug, Args, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ShowArgs {
    /// Keep only events whose dotted-kebab id starts with this
    /// prefix (e.g. `slice.build` or `plan.entry.advanced`).
    #[arg(long)]
    pub filter: Option<String>,

    /// Keep only the most recent N matching events.
    #[arg(long)]
    pub limit: Option<usize>,
}
