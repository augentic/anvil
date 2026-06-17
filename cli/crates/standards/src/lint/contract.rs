//! Binary-injected CLI contract DTOs.
//!
//! [`CliContract`] is the machine-readable surface `specify contract
//! dump` emits and the `cli-contract` hint kind checks documentation
//! against. The standards layer owns only the *shape*: the root binary
//! populates it (clap introspection plus the const tables in
//! `specify-error`, `specify-workflow`, and `specify-schema`) and
//! injects it into the lint pipeline, preserving the
//! standards⊥workflow dependency invariant.

use serde::{Deserialize, Serialize};

/// The machine-readable contract one `specify` binary exposes.
///
/// Round-trips `schemas/contract/dump.schema.json`
/// (`CONTRACT_DUMP_JSON_SCHEMA`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct CliContract {
    /// Contract payload shape version (`1`).
    pub version: u32,
    /// `CARGO_PKG_VERSION` of the emitting binary.
    pub binary_version: String,
    /// Root of the clap verb tree (`name: "specify"`).
    pub commands: CommandNode,
    /// The closed process exit-code table.
    pub exit_codes: Vec<ExitCode>,
    /// Stable kebab-case error discriminants (`specify_error::codes`).
    pub error_ids: Vec<String>,
    /// Closed journal event ids (`specify_workflow::journal::WIRE_EVENT_IDS`).
    pub journal_event_ids: Vec<String>,
    /// `schemas/`-relative paths of every embedded JSON Schema.
    pub schemas: Vec<String>,
    /// Workspace-relative paths of every file under the binary's
    /// `tests/` tree, embedded at build time (named-test citation
    /// checking). Empty when the build had no test tree.
    #[serde(default)]
    pub tests: Vec<String>,
}

/// One verb in the clap tree: its name, the flag/positional argument
/// surface, and nested subcommands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct CommandNode {
    /// Verb name as typed on the command line.
    pub name: String,
    /// First-line help text, when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub about: Option<String>,
    /// Argument surface: `--long` for flags, `<id>` for positionals.
    pub args: Vec<String>,
    /// Nested subcommands, in declaration order.
    pub subcommands: Vec<Self>,
}

/// One row of the closed exit-code table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ExitCode {
    /// Numeric process exit code.
    pub code: u8,
    /// Stable kebab-case name (e.g. `validation-failed`).
    pub name: String,
    /// One-line meaning.
    pub meaning: String,
}
