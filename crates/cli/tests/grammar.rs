//! In-process coverage of the shared grammar seam: the [`parse`]
//! entry point's exit-code contract and the clap-to-`Input`
//! conversions the shim dispatch matches rely on.
//!
//! The full per-verb behaviour (filesystem effects, typed failures)
//! is driven through the `Handler` layer by `crates/workflow/tests`;
//! these tests pin the argv boundary the shims depend on.

use cli::cli::{Cli, Commands, parse};
use cli::commands::plan::cli::PlanAction;
use cli::commands::plan::source_map;

/// Parse one argv line (program name included) through the shared
/// grammar, panicking on parse failure.
fn parse_ok(argv: &[&str]) -> Cli {
    parse(argv.iter().map(ToString::to_string)).unwrap_or_else(|exit| {
        panic!("argv {argv:?} failed to parse (exit {})", exit.code());
    })
}

#[test]
fn parse_maps_help_to_exit_zero() {
    let exit = parse(["specify", "--help"].map(String::from)).expect_err("--help short-circuits");
    assert_eq!(exit.code(), 0);
}

#[test]
fn parse_maps_usage_error_to_exit_two() {
    let exit =
        parse(["specify", "--no-such-flag"].map(String::from)).expect_err("unknown flag fails");
    assert_eq!(exit.code(), 2, "clap usage errors exit 2 on every shim");
}

#[test]
fn parse_replaces_argv0() {
    // A host may supply argv[0] as the deployment's guest id; the
    // grammar must parse regardless because parse() re-stamps it.
    let cli = parse_ok(&["workflow", "plan", "status"]);
    assert!(
        matches!(
            cli.command,
            Commands::Plan {
                action: PlanAction::Status
            }
        ),
        "argv[0] is replaced before parsing"
    );
}

#[test]
fn author_sources_desugar() {
    // `--source` + `--intent` desugar through the same source_map the
    // dispatch matches use for `plan create` and `plan author`.
    let cli = parse_ok(&[
        "specify",
        "plan",
        "author",
        "account-revamp",
        "--source",
        "docs=documentation:./design-notes",
        "--intent",
        "revamp the account area",
    ]);
    let Commands::Plan {
        action: PlanAction::Author { sources, intent, .. },
    } = cli.command
    else {
        panic!("plan author parses to its action");
    };
    let map = source_map(sources, intent).expect("bindings desugar");
    assert_eq!(map.len(), 2, "docs plus the desugared intent binding");
    assert_eq!(map["intent"].adapter, "intent");
    assert_eq!(map["intent"].value.as_deref(), Some("revamp the account area"));
    assert_eq!(map["docs"].adapter, "documentation");
    assert_eq!(map["docs"].path.as_deref(), Some("./design-notes"));
}

#[test]
fn duplicate_intent_binding_refused() {
    let cli = parse_ok(&[
        "specify",
        "plan",
        "author",
        "account-revamp",
        "--source",
        "intent=intent:value:explicit",
        "--intent",
        "sugared",
    ]);
    let Commands::Plan {
        action: PlanAction::Author { sources, intent, .. },
    } = cli.command
    else {
        panic!("plan author parses to its action");
    };
    let err = source_map(sources, intent).expect_err("duplicate intent key refused");
    assert!(
        matches!(err, error::Error::Diag { code, .. } if code == "plan-source-duplicate-key"),
        "the duplicate-key gate names its stable discriminant"
    );
}
