//! Clap derive surface for `specify extension *`. The umbrella `cli.rs`
//! re-exports `ExtensionAction`.

use clap::Subcommand;

#[derive(Subcommand)]
pub enum ExtensionAction {
    /// Fetch if needed, then run a declared WASI tool.
    Run {
        /// Declared tool name.
        name: String,
        /// Args forwarded to the tool after `--`.
        #[arg(last = true)]
        args: Vec<String>,
    },
    /// Fetch one declared tool, or every declared tool when omitted.
    Fetch {
        /// Optional declared tool name to fetch.
        name: Option<String>,
    },
    /// Remove unused cache entries for the current project.
    Gc,
    /// Print a tool-owned schema by name (delegates to the tool's
    /// `schema` subcommand).
    Schema {
        /// Declared tool name (e.g. `vectis`, `contract`).
        name: String,
        /// Schema name within the tool (e.g. `tokens`, `assets`).
        schema: String,
    },
}
