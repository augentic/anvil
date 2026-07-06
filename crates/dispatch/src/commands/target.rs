//! `specify target {resolve}` — target adapter operations.
//!
//! Target adapters carry `axis: target` and the `shape` + `build` +
//! `merge` capabilities. The run-side dispatch lives on the unified
//! `commands::resolve_plugin` helper (it is byte-identical to the
//! source-axis path apart from the `@version` peel); this module only
//! owns the clap derive surface under [`cli`].

pub mod cli;
