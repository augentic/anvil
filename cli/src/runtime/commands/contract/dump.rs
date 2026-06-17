//! `specify contract dump` handler: assemble and emit the
//! [`CliContract`] payload.

use std::io::Write;

use clap::CommandFactory;
use specify_error::Result;
use specify_standards::lint::contract::{CliContract, CommandNode, ExitCode};

use crate::runtime::cli::Cli;
use crate::runtime::output::{EXIT_CODES, Format, emit};

/// Contract payload shape version pinned by
/// `schemas/contract/dump.schema.json`.
const CONTRACT_VERSION: u32 = 1;

/// Build-time inventory of the `tests/` tree (newline-delimited
/// workspace-relative paths), assembled by `build.rs`.
const TESTS_INVENTORY: &str = include_str!(concat!(env!("OUT_DIR"), "/tests-inventory.txt"));

/// Assemble the live [`CliContract`] for this binary.
///
/// Shared by the `contract dump` handler and the `specify lint
/// framework` cross-check so both surfaces see the identical contract.
pub fn build_contract() -> CliContract {
    CliContract {
        version: CONTRACT_VERSION,
        binary_version: env!("CARGO_PKG_VERSION").to_string(),
        commands: command_node(&Cli::command()),
        exit_codes: EXIT_CODES
            .iter()
            .map(|(code, name, meaning)| ExitCode {
                code: *code,
                name: (*name).to_string(),
                meaning: (*meaning).to_string(),
            })
            .collect(),
        error_ids: specify_error::codes::WIRE_CODES.iter().map(ToString::to_string).collect(),
        journal_event_ids: specify_workflow::journal::WIRE_EVENT_IDS
            .iter()
            .map(ToString::to_string)
            .collect(),
        schemas: specify_schema::EMBEDDED_SCHEMAS
            .iter()
            .map(|(_, path, _)| (*path).to_string())
            .collect(),
        tests: TESTS_INVENTORY.lines().map(ToString::to_string).collect(),
    }
}

/// Project one clap [`clap::Command`] (and its subtree) into the wire
/// [`CommandNode`]. Flags render as `--long`, positionals as `<id>`;
/// clap's auto `help`/`version` args and the implicit `help`
/// subcommand are elided — they are clap furniture, not contract.
fn command_node(cmd: &clap::Command) -> CommandNode {
    let args = cmd
        .get_arguments()
        .filter(|arg| {
            let id = arg.get_id().as_str();
            id != "help" && id != "version"
        })
        .map(|arg| {
            arg.get_long()
                .map_or_else(|| format!("<{}>", arg.get_id().as_str()), |long| format!("--{long}"))
        })
        .collect();
    let subcommands =
        cmd.get_subcommands().filter(|sub| sub.get_name() != "help").map(command_node).collect();
    CommandNode {
        name: cmd.get_name().to_string(),
        about: cmd.get_about().map(ToString::to_string),
        args,
        subcommands,
    }
}

/// Emit the contract on stdout in the requested format.
///
/// # Errors
///
/// Propagates the serialisation or I/O failure from [`emit`].
pub fn run(format: Format) -> Result<()> {
    let contract = build_contract();
    emit(&mut std::io::stdout().lock(), format, &contract, write_text)?;
    Ok(())
}

fn write_text(w: &mut dyn Write, contract: &CliContract) -> std::io::Result<()> {
    writeln!(w, "specify {}", contract.binary_version)?;
    writeln!(w, "  verbs: {}", count_verbs(&contract.commands))?;
    writeln!(w, "  exit-codes: {}", contract.exit_codes.len())?;
    writeln!(w, "  error-ids: {}", contract.error_ids.len())?;
    writeln!(w, "  journal-event-ids: {}", contract.journal_event_ids.len())?;
    writeln!(w, "  schemas: {}", contract.schemas.len())?;
    writeln!(w, "  tests: {}", contract.tests.len())?;
    writeln!(w, "(run with --format json for the full contract)")?;
    Ok(())
}

/// Count every node in the verb tree, the root included.
fn count_verbs(node: &CommandNode) -> usize {
    1 + node.subcommands.iter().map(count_verbs).sum::<usize>()
}

#[cfg(test)]
mod tests {
    use super::build_contract;

    #[test]
    fn contract_covers_known_surface() {
        let contract = build_contract();
        assert_eq!(contract.commands.name, "specify");
        let top: Vec<&str> =
            contract.commands.subcommands.iter().map(|s| s.name.as_str()).collect();
        for verb in ["init", "plan", "slice", "journal", "lint", "contract", "completions"] {
            assert!(top.contains(&verb), "top-level verb `{verb}` missing from {top:?}");
        }
        assert!(
            contract.commands.args.contains(&"--format".to_string()),
            "global --format flag must be on the root node: {:?}",
            contract.commands.args
        );
        let plan = contract
            .commands
            .subcommands
            .iter()
            .find(|s| s.name == "plan")
            .expect("plan subcommand present");
        assert!(
            plan.subcommands.iter().any(|s| s.name == "next"),
            "plan next missing from {:?}",
            plan.subcommands.iter().map(|s| &s.name).collect::<Vec<_>>()
        );
        assert!(contract.error_ids.iter().any(|id| id == "adapter-not-found"));
        assert!(contract.journal_event_ids.iter().any(|id| id == "plan.transition.approved"));
        assert!(contract.schemas.iter().any(|p| p == "schemas/contract/dump.schema.json"));
        assert!(
            contract.tests.iter().any(|p| p == "tests/cli_contract.rs"),
            "tests inventory must carry the contract test itself"
        );
    }
}
