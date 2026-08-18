//! Typed command grammar, conversion, and HTTP parity coverage over
//! the pruned two-verb surface (ADR-0008 §3): `init`, the `specify`
//! stub, and the auto-derived `completions`.

use std::fs;
use std::path::PathBuf;

use native::{DynModel, Provider};
use omnia_guest::api::invoke::Invoker;
use omnia_testkit::model::Harness;

// Grammar and parity coverage only: no test dispatches judgment or an
// adapter seam, so the scripted provider's empty script never runs and
// the explicit inert locations are never resolved.
fn provider(root: impl Into<PathBuf>) -> Provider {
    let root = root.into();
    let locations = project::handler::Locations::explicit(
        root.join("store"),
        project::handler::CachePlacement::Parent(root.join("project-cache")),
    );
    Provider::new(
        project::handler::ExecutionPaths::new(root, locations),
        DynModel::new(Harness::answering(Vec::<String>::new())),
        mock::catalog(),
    )
}

fn command_router(
    root: impl Into<PathBuf>,
) -> omnia_guest::api::command::Router<Provider, transport::command::Globals> {
    transport::command::router(Invoker::new("emery", provider(root))).expect("router")
}

// C3 tripwire: the guest HTTP listener serves only the MCP reference
// shelves — the engine's whole non-shelf surface is one typed refusal
// with no operation route table. If an HTTP operation surface returns,
// it must arrive with an authenticated operator ingress design
// (target-architecture §7) and replace this test deliberately.
#[tokio::test]
async fn http_parity() {
    use omnia_guest::http::{Method, Request, StatusCode};
    use tower::ServiceExt as _;

    // Every command-router verb, projected as an HTTP-ish path, must
    // refuse — derived from the live command inventory so a new verb
    // can never quietly gain an HTTP twin.
    let command = command_router(".");
    for route in command.inventory() {
        let path = format!("/{}", route.selector().path().join("/"));
        for method in [Method::GET, Method::POST] {
            let request = Request::builder()
                .method(method.clone())
                .uri(&path)
                .body(omnia_guest::axum::body::Body::empty())
                .expect("build request");
            let response = transport::http::refusal()
                .oneshot(request)
                .await
                .expect("refusal serves the request");
            assert_eq!(response.status(), StatusCode::NOT_FOUND, "{method} {path} must refuse");
        }
    }
}

// ADR-0008 §3: the grammar is `init` + `specify` + `completions` and
// nothing else — deleted verbs are deletions from the grammar, not
// hidden routes or "deprecated" stubs.
#[tokio::test]
async fn pruned_grammar() {
    let router = command_router(".");

    let inventory: Vec<Vec<String>> =
        router.inventory().iter().map(|route| route.selector().path().to_vec()).collect();
    assert_eq!(
        inventory,
        [vec!["completions".to_string()], vec!["init".to_string()], vec!["specify".to_string()]]
    );

    for removed in [
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
    assert!(help.contains("init"), "{help}");
    assert!(help.contains("specify"), "{help}");
    for gone in ["plan", "slice", "system", "journal", "debt", "adapter"] {
        assert!(
            !help.lines().any(|line| line.trim_start().starts_with(gone)),
            "help must not list `{gone}`: {help}"
        );
    }
}

// The reserved verb parses and fails typed — no orchestration, no
// output-home scaffolding, no artifacts.
#[tokio::test]
async fn specify_stub() {
    let home = tempfile::tempdir().expect("tempdir");
    let router = command_router(home.path());

    let response = router.execute(["emery", "specify"]).await;
    assert_eq!(response.exit, 1);
    let stderr = String::from_utf8_lossy(&response.stderr);
    assert!(stderr.contains("specify"), "{stderr}");

    let json = router.execute(["emery", "--format", "json", "specify"]).await;
    assert_eq!(json.exit, 1);
    let stderr = String::from_utf8(json.stderr).expect("stderr utf-8");
    let envelope: serde_json::Value = serde_json::from_str(&stderr).expect("one JSON envelope");
    assert_eq!(envelope["error"], "specify-not-implemented");
    assert_eq!(envelope["exit-code"], 1);

    assert!(fs::read_dir(home.path()).expect("home").next().is_none(), "the stub writes nothing");
}

#[tokio::test]
async fn globals_and_completions() {
    let router = command_router(".");

    let completions = router.execute(["emery", "completions", "zsh"]).await;
    assert_eq!(completions.exit, 0);
    assert!(!completions.stdout.is_empty());
    let completion_help = router.execute(["emery", "completions", "--help"]).await;
    let completion_help = String::from_utf8_lossy(&completion_help.stdout);
    assert!(completion_help.contains("Pipe into your shell's completion directory"));
    assert!(completion_help.contains("output tracks the live clap surface"));
}

#[tokio::test]
async fn version_host_semver() {
    // No adapter-train suffix: adapters version independently and
    // resolve local-first, so the binary reports only its own SemVer.
    let router = command_router(".");
    let response = router.execute(["emery", "--version"]).await;
    assert_eq!(response.exit, 0);
    let stdout = String::from_utf8_lossy(&response.stdout);
    let expected = format!("emery {}", env!("CARGO_PKG_VERSION"));
    assert!(stdout.trim_end().ends_with(&expected), "{stdout}");
}

#[tokio::test]
async fn argv_zero_replaced() {
    let router = command_router(".");
    let expected = router.execute(["emery", "init", "--no-such-flag"]).await;
    let forwarded = router.execute(["emery:engine@0.1.0", "init", "--no-such-flag"]).await;

    assert_eq!(expected.exit, 2);
    assert_eq!(forwarded.exit, expected.exit);
    assert_eq!(forwarded.stderr, expected.stderr);
    let stderr = String::from_utf8_lossy(&forwarded.stderr);
    assert!(stderr.contains("Usage: emery init"), "{stderr}");
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
            name: "init adapter required",
            argv: &["emery", "init"],
            exit: 2,
            stdout: "",
            stderr: "init-adapter-required",
            json_channels: false,
        },
        Case {
            name: "specify stub",
            argv: &["emery", "--format", "json", "specify"],
            exit: 1,
            stdout: "",
            stderr: "specify-not-implemented",
            json_channels: true,
        },
    ]
}

#[tokio::test]
async fn native_response_contract() {
    for case in cases() {
        // A bare uninitialized tempdir: no case needs a scaffolded
        // project, and `init` without an adapter must refuse rather
        // than take the already-initialized re-entry path.
        let project = tempfile::tempdir().expect("tempdir");
        let response = command_router(project.path()).execute(case.argv).await;
        let stdout = String::from_utf8(response.stdout).expect("stdout is UTF-8");
        let stderr = String::from_utf8(response.stderr).expect("stderr is UTF-8");

        assert_eq!(response.exit, case.exit, "{} exit", case.name);
        assert!(stdout.contains(case.stdout), "{} stdout: {stdout}", case.name);
        assert!(stderr.contains(case.stderr), "{} stderr: {stderr}", case.name);
        if case.json_channels {
            if !stdout.is_empty() {
                serde_json::from_str::<serde_json::Value>(&stdout)
                    .unwrap_or_else(|error| panic!("{} stdout JSON: {error}", case.name));
            }
            if !stderr.is_empty() {
                serde_json::from_str::<serde_json::Value>(&stderr)
                    .unwrap_or_else(|error| panic!("{} stderr JSON: {error}", case.name));
            }
        }
    }
}
