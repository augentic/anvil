//! The CLI wire contract over an idle provider: the route budget,
//! grammar failures, exit codes, and the text/JSON channel shape.
//! Capabilities are never dispatched — every case fails (or succeeds)
//! before the engine reaches a model or a source.

#![cfg(not(target_arch = "wasm32"))]

mod support;

use serde_json::Value;
use support::{Provider, cli, cli_ok, router};

// Widening the route budget requires an ADR (ADR-0008); deleted verbs
// are deleted from the grammar, not hidden.
#[tokio::test]
async fn route_budget() {
    let provider = Provider::idle();
    let router = router(&provider);

    let inventory: Vec<Vec<String>> =
        router.inventory().iter().map(|route| route.selector().path().to_vec()).collect();
    assert_eq!(
        inventory,
        [
            Vec::from(["completions"].map(str::to_string)),
            Vec::from(["show"].map(str::to_string)),
            Vec::from(["specify"].map(str::to_string)),
        ]
    );

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
        assert_eq!(router.execute(removed.iter().copied()).await.exit, 2, "{removed:?}");
    }

    let help = router.execute(["emery", "--help"]).await;
    assert_eq!(help.exit, 0);
    let help = String::from_utf8_lossy(&help.stdout);
    assert!(help.contains("specify"), "{help}");
    assert!(help.contains("show"), "{help}");
    for gone in ["init", "plan", "slice", "system", "journal", "debt", "adapter"] {
        assert!(
            !help.lines().any(|line| line.trim_start().starts_with(gone)),
            "help must not list `{gone}`: {help}"
        );
    }
}

// `specify` requires at least one source; a refused run writes nothing.
#[tokio::test]
async fn specify_without_sources() {
    let provider = Provider::idle();

    let response = cli(&provider, &["emery", "specify"]).await;
    assert_eq!(response.exit, 2);
    let stderr = String::from_utf8_lossy(&response.stderr);
    assert!(stderr.contains("at least one source"), "{stderr}");

    fail(&provider, &["emery", "specify"], 2, "specify-source-required").await;
    assert!(provider.storage.is_empty(), "a refused run writes nothing");
}

// `--sources` carries the whole binding list; mixing refuses typed.
#[tokio::test]
async fn specify_mixed_sources() {
    let provider = Provider::idle();

    for argv in [
        &["emery", "specify", "docs", "--sources", "sources.toml"][..],
        &["emery", "specify", "--value", "intent=text", "--sources", "sources.toml"][..],
    ] {
        fail(&provider, argv, 2, "argument").await;
    }

    assert!(provider.storage.is_empty(), "a refused run writes nothing");
}

// Each source binds once; a repeated key refuses typed.
#[tokio::test]
async fn specify_duplicate_source() {
    let provider = Provider::idle();
    fail(&provider, &["emery", "specify", "docs", "docs"], 2, "specify-source-duplicate").await;
    assert!(provider.storage.is_empty(), "a refused run writes nothing");
}

// `--value` needs the `<adapter>=<text>` shape.
#[tokio::test]
async fn specify_malformed_value() {
    let provider = Provider::idle();
    fail(&provider, &["emery", "specify", "--value", "no-equals"], 2, "argument").await;
}

// The read verb fails typed before any generation is committed.
#[tokio::test]
async fn show_without_generation() {
    let provider = Provider::idle();

    let response = cli(&provider, &["emery", "show", "spec"]).await;
    assert_eq!(response.exit, 1);
    let stderr = String::from_utf8_lossy(&response.stderr);
    assert!(stderr.contains("spec-not-generated"), "{stderr}");

    fail(&provider, &["emery", "show", "design"], 1, "spec-not-generated").await;
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
    assert!(help.contains("output tracks the live clap surface"));
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

    assert_eq!(expected.exit, 2);
    assert_eq!(forwarded.exit, expected.exit);
    assert_eq!(forwarded.stderr, expected.stderr);
    let stderr = String::from_utf8_lossy(&forwarded.stderr);
    assert!(stderr.contains("Usage: emery specify"), "{stderr}");
    assert!(!stderr.contains("emery:engine@0.1.0"));
}

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
            exit: 2,
            stdout: "",
            stderr: "specify-source-required",
            json_channels: false,
        },
        Case {
            name: "show not generated",
            argv: &["emery", "--format", "json", "show", "spec"],
            exit: 1,
            stdout: "",
            stderr: "spec-not-generated",
            json_channels: true,
        },
    ]
}

// The stdout/stderr channel contract, table-driven across the surface.
#[tokio::test]
async fn response_contract() {
    for case in cases() {
        // A fresh store keeps `specify` sourceless and `show` without a generation.
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

// Runs `argv` in JSON mode and asserts the typed failure envelope.
async fn fail(provider: &Provider, argv: &[&str], exit: u8, code: &str) {
    let mut json = vec!["emery", "--format", "json"];
    json.extend(argv.iter().skip(1).copied());
    let resp = cli(provider, &json).await;
    assert_eq!(resp.exit, exit, "{code}: {}", String::from_utf8_lossy(&resp.stderr));
    let envelope: Value = serde_json::from_slice(&resp.stderr).expect("one JSON envelope");
    assert_eq!(envelope["error"], code, "{envelope}");
    assert_eq!(envelope["exit-code"], exit, "{envelope}");
}
