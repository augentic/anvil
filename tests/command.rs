//! CLI wire contract
//!
//! What the `emery` command promises regardless of what the engine does:
//! which verbs exist, how grammar mistakes are reported, which exit code each
//! failure class maps to, and what lands on stdout versus stderr in text and
//! JSON mode.
//!
//! Every scenario finishes before the engine touches a model or a source, so
//! the provider here is idle. That keeps these cases about the command
//! surface alone; the product behaviour behind it is `specify.rs`.

#![cfg(not(target_arch = "wasm32"))]

mod support;
#[path = "support/verbs.rs"]
mod verbs;

use omnia_guest::api::command::USAGE_EXIT;
use serde_json::Value;
use support::{Provider, cli, cli_ok, fail};
use verbs::verbs;

struct Case {
    name: &'static str,
    argv: &'static [&'static str],
    exit: u8,
    stdout: &'static str,
    stderr: &'static str,
    json_channels: bool,
}

const fn cases() -> [Case; 5] {
    [
        Case {
            name: "help",
            argv: &["emery", "--help"],
            exit: 0,
            stdout: "Usage: emery [OPTIONS] <COMMAND>",
            stderr: "",
            json_channels: false,
        },
        Case {
            name: "version",
            argv: &["emery", "--version"],
            exit: 0,
            stdout: concat!("emery ", env!("CARGO_PKG_VERSION")),
            stderr: "",
            json_channels: false,
        },
        Case {
            name: "completions",
            argv: &["emery", "completions", "zsh"],
            exit: 0,
            stdout: "_emery",
            stderr: "",
            json_channels: false,
        },
        Case {
            name: "specify source required",
            argv: &["emery", "specify"],
            exit: 1,
            stdout: "",
            stderr: "specify-source-required",
            json_channels: false,
        },
        Case {
            name: "show not generated",
            argv: &["emery", "--format", "json", "show", "spec"],
            exit: 2,
            stdout: "",
            stderr: "spec-not-generated",
            json_channels: true,
        },
    ]
}

// Deleted verbs are deleted from the grammar, not hidden. A usage error
// exits `USAGE_EXIT` (64), so exit 2 always means a `NotFound` envelope.
#[tokio::test]
async fn route_budget() {
    let provider = Provider::idle();

    for removed in [
        &["emery", "init"][..],
        &["emery", "plan", "status"][..],
        &["emery", "plan", "author"][..],
        &["emery", "plan", "refine"][..],
        &["emery", "plan", "execute"][..],
        &["emery", "plan", "archive"][..],
        &["emery", "slice", "list"][..],
        &["emery", "slice", "validate"][..],
        &["emery", "source", "survey"][..],
        &["emery", "source", "extract"][..],
        &["emery", "source", "resolve"][..],
        &["emery", "target", "resolve"][..],
        &["emery", "system", "survey"][..],
        &["emery", "system", "plan"][..],
        &["emery", "system", "review"][..],
        &["emery", "system", "status"][..],
        &["emery", "adapter", "add"][..],
        &["emery", "adapter", "upgrade"][..],
        &["emery", "archive", "prune"][..],
        &["emery", "journal", "show"][..],
        &["emery", "debt"][..],
    ] {
        assert_eq!(cli(&provider, removed).await.exit, USAGE_EXIT, "{removed:?}");
    }

    let help = cli(&provider, &["emery", "--help"]).await;
    assert_eq!(help.exit, 0);
    let help = String::from_utf8_lossy(&help.stdout);
    assert_eq!(verbs(&help), ["completions", "show", "specify"]);
    for gone in ["init", "plan", "slice", "system", "journal", "debt", "adapter"] {
        assert!(
            !help.lines().any(|line| line.trim_start().starts_with(gone)),
            "help must not list `{gone}`: {help}"
        );
    }
}

// A run naming no sources discovers the project-root `emery.toml`; with no
// file to discover it refuses typed and writes nothing. The CWD move
// is safe under nextest's process-per-test isolation.
#[tokio::test]
async fn specify_without_sources() {
    let dir = tempfile::TempDir::new().expect("tempdir");
    std::env::set_current_dir(dir.path()).expect("enter empty project");
    let provider = Provider::idle();

    let response = cli(&provider, &["emery", "specify"]).await;
    assert_eq!(response.exit, 1);
    let stderr = String::from_utf8_lossy(&response.stderr);
    assert!(stderr.contains("no sources"), "{stderr}");

    fail(&provider, &["emery", "specify"], 1, "specify-source-required").await;
    assert!(provider.storage.is_empty(), "a refused run writes nothing");
}

// Naming the file carrier without a value explicitly selects the
// project-relative `emery.toml`; a missing explicit file is a read
// error, never a discovery miss.
#[tokio::test]
async fn specify_default_config_path() {
    let dir = tempfile::TempDir::new().expect("tempdir");
    std::env::set_current_dir(dir.path()).expect("enter empty project");
    let provider = Provider::idle();

    let response = cli(&provider, &["emery", "specify", "--config"]).await;
    assert_eq!(response.exit, 3);
    let stderr = String::from_utf8_lossy(&response.stderr);
    assert!(stderr.contains("emery.toml"), "{stderr}");
    assert!(provider.storage.is_empty(), "a refused run writes nothing");
}

// `--config` carries the whole source list; mixing refuses typed.
#[tokio::test]
async fn specify_mixed_sources() {
    let provider = Provider::idle();

    for argv in [
        &["emery", "specify", "docs", "--config", "emery.toml"][..],
        &["emery", "specify", "--description", "intent=text", "--config", "emery.toml"][..],
    ] {
        fail(&provider, argv, 1, "bad_request").await;
    }

    assert!(provider.storage.is_empty(), "a refused run writes nothing");
}

// Each source binds once; a repeated key refuses typed whichever
// carrier repeats it.
#[tokio::test]
async fn specify_duplicate_source() {
    let provider = Provider::idle();
    for argv in [
        &["emery", "specify", "docs", "docs"][..],
        &["emery", "specify", "docs", "--description", "docs=inline text"][..],
    ] {
        fail(&provider, argv, 1, "bad_request").await;
    }
    assert!(provider.storage.is_empty(), "a refused run writes nothing");
}

// `--description` needs the `<adapter>=<text>` shape.
#[tokio::test]
async fn specify_malformed_description() {
    let provider = Provider::idle();
    fail(&provider, &["emery", "specify", "--description", "no-equals"], 1, "bad_request").await;
}

// The superseded flag spellings are deleted from the grammar, not
// aliased (hard cut): clap refuses them as unknown arguments.
#[tokio::test]
async fn specify_old_flags_deleted() {
    let provider = Provider::idle();
    for argv in [
        &["emery", "specify", "--sources", "emery.toml"][..],
        &["emery", "specify", "--value", "intent=text"][..],
    ] {
        assert_eq!(cli(&provider, argv).await.exit, USAGE_EXIT, "{argv:?}");
    }
}

// The read verb fails typed before any revision is committed.
#[tokio::test]
async fn show_without_revision() {
    let provider = Provider::idle();

    let response = cli(&provider, &["emery", "show", "spec"]).await;
    assert_eq!(response.exit, 2);
    let stderr = String::from_utf8_lossy(&response.stderr);
    assert!(stderr.contains("spec-not-generated"), "{stderr}");

    fail(&provider, &["emery", "show", "design"], 2, "spec-not-generated").await;
    assert!(provider.storage.is_empty(), "a refused read writes nothing");
}

#[tokio::test]
async fn globals_and_completions() {
    let provider = Provider::idle();

    let completions = cli_ok(&provider, &["emery", "completions", "zsh"]).await;
    assert!(!completions.stdout.is_empty());
    let help = cli_ok(&provider, &["emery", "completions", "--help"]).await;
    let help = String::from_utf8_lossy(&help.stdout);
    assert!(help.contains("Pipe into your shell's completion directory"));
    assert!(help.contains("emery completions zsh > ~/.zsh/_emery"));
}

// Adapters version independently, so the binary reports its own SemVer.
#[tokio::test]
async fn version_host_semver() {
    let provider = Provider::idle();
    let response = cli_ok(&provider, &["emery", "--version"]).await;
    let stdout = String::from_utf8_lossy(&response.stdout);
    let expected = format!("emery {}", env!("CARGO_PKG_VERSION"));
    assert!(stdout.trim_end().ends_with(&expected), "{stdout}");
}

// Omnia forwards raw argv; a routed-id argv[0] renders as `emery`.
#[tokio::test]
async fn argv_zero_replaced() {
    let provider = Provider::idle();
    let expected = cli(&provider, &["emery", "specify", "--no-such-flag"]).await;
    let forwarded = cli(&provider, &["emery:engine@0.1.0", "specify", "--no-such-flag"]).await;

    assert_eq!(expected.exit, USAGE_EXIT);
    assert_eq!(forwarded.exit, expected.exit);
    assert_eq!(forwarded.stderr, expected.stderr);
    let stderr = String::from_utf8_lossy(&forwarded.stderr);
    assert!(stderr.contains("Usage: emery specify"), "{stderr}");
    assert!(!stderr.contains("emery:engine@0.1.0"));
}

// The stdout/stderr channel contract, table-driven across the surface.
#[tokio::test]
async fn response_contract() {
    for case in cases() {
        // A fresh store keeps `specify` sourceless and `show` without a revision.
        let response = cli(&Provider::idle(), case.argv).await;
        let stdout = String::from_utf8(response.stdout).expect("stdout is UTF-8");
        let stderr = String::from_utf8(response.stderr).expect("stderr is UTF-8");

        assert_eq!(response.exit, case.exit, "{} exit", case.name);
        assert!(stdout.contains(case.stdout), "{} stdout: {stdout}", case.name);
        assert!(stderr.contains(case.stderr), "{} stderr: {stderr}", case.name);
        if case.json_channels {
            if !stdout.is_empty() {
                serde_json::from_str::<Value>(&stdout)
                    .unwrap_or_else(|error| panic!("{} stdout JSON: {error}", case.name));
            }
            if !stderr.is_empty() {
                serde_json::from_str::<Value>(&stderr)
                    .unwrap_or_else(|error| panic!("{} stderr JSON: {error}", case.name));
            }
        }
    }
}
