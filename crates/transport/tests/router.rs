//! Typed command grammar, conversion, and HTTP parity coverage over
//! the two-verb surface: `init`, the live `specify` generator, and
//! the auto-derived `completions`.

mod support;

// Grammar and parity coverage only: no test dispatches judgment or an
// adapter seam, so the inert provider's capabilities are never reached.
fn command_router()
-> omnia_guest::api::command::Router<support::Inert, emery_transport::command::Globals> {
    support::router()
}

// Gate tripwire: the guest HTTP listener serves only the MCP shelves;
// an HTTP operation surface must arrive with an authenticated ingress
// design and delete this test in the same decision.
#[tokio::test]
async fn adr_0002_http_refusal() {
    use omnia_guest::http::{Method, Request, StatusCode};
    use tower::ServiceExt as _;

    // Every command-router verb, projected as an HTTP-ish path, must
    // refuse — derived from the live command inventory so a new verb
    // can never quietly gain an HTTP twin.
    let command = command_router();
    for route in command.inventory() {
        let path = format!("/{}", route.selector().path().join("/"));
        for method in [Method::GET, Method::POST] {
            let request = Request::builder()
                .method(method.clone())
                .uri(&path)
                .body(omnia_guest::axum::body::Body::empty())
                .expect("build request");
            let response = emery_transport::http::refusal()
                .oneshot(request)
                .await
                .expect("refusal serves the request");
            assert_eq!(response.status(), StatusCode::NOT_FOUND, "{method} {path} must refuse");
        }
    }
}

// Gate tripwire: the route budget is the live verb list and nothing
// else; widening it requires an ADR.
#[tokio::test]
async fn adr_0008_route_budget() {
    let router = command_router();

    let inventory: Vec<Vec<String>> =
        router.inventory().iter().map(|route| route.selector().path().to_vec()).collect();
    assert_eq!(
        inventory,
        [
            Vec::from(["completions"].map(str::to_string)),
            Vec::from(["init"].map(str::to_string)),
            Vec::from(["specify"].map(str::to_string)),
        ]
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

// The live generator fails closed outside an initialised project — no
// orchestration, no output-home writes, no artifacts.
#[tokio::test]
async fn specify_uninitialized() {
    let provider = support::Inert::default();
    let storage = std::sync::Arc::clone(&provider.storage);
    let router = support::router_over(provider);

    let response = router.execute(["emery", "specify"]).await;
    assert_eq!(response.exit, 1);
    let stderr = String::from_utf8_lossy(&response.stderr);
    assert!(stderr.contains("not-initialized"), "{stderr}");

    let json = router.execute(["emery", "--format", "json", "specify"]).await;
    assert_eq!(json.exit, 1);
    let stderr = String::from_utf8(json.stderr).expect("stderr utf-8");
    let envelope: serde_json::Value = serde_json::from_str(&stderr).expect("one JSON envelope");
    assert_eq!(envelope["error"], "not-initialized");
    assert_eq!(envelope["exit-code"], 1);

    assert!(storage.is_empty(), "a refused run writes nothing");
}

#[tokio::test]
async fn globals_and_completions() {
    let router = command_router();

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
    let router = command_router();
    let response = router.execute(["emery", "--version"]).await;
    assert_eq!(response.exit, 0);
    let stdout = String::from_utf8_lossy(&response.stdout);
    let expected = format!("emery {}", env!("CARGO_PKG_VERSION"));
    assert!(stdout.trim_end().ends_with(&expected), "{stdout}");
}

#[tokio::test]
async fn argv_zero_replaced() {
    let router = command_router();
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
            name: "init source required",
            argv: &["emery", "init"],
            exit: 2,
            stdout: "",
            stderr: "init-source-required",
            json_channels: false,
        },
        Case {
            name: "specify uninitialized",
            argv: &["emery", "--format", "json", "specify"],
            exit: 1,
            stdout: "",
            stderr: "not-initialized",
            json_channels: true,
        },
    ]
}

#[tokio::test]
async fn native_response_contract() {
    for case in cases() {
        // Each case runs over a fresh, empty scripted store: `init`
        // without an adapter must refuse rather than take the
        // re-entry path, and `specify` must fail `not-initialized`.
        let response = command_router().execute(case.argv).await;
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
