//! Clap derive surface for `specify contract *`. The umbrella `cli.rs`
//! re-exports `ContractAction`.

use clap::Subcommand;

#[derive(Subcommand)]
pub enum ContractAction {
    /// Emit the machine-readable CLI contract.
    ///
    /// The payload (pinned by `schemas/contract/dump.schema.json`)
    /// carries the binary version, the full verb tree with flags, the
    /// closed exit-code table, the stable kebab-case error
    /// discriminants, the closed journal event-id taxonomy, and the
    /// embedded JSON Schema paths. Read-only and project-context-free;
    /// `specify lint framework` consumes the same contract to
    /// cross-check documented invocations against the live surface.
    Dump,
}
