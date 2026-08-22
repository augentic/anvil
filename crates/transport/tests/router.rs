//! Command grammar, conversion, and HTTP parity coverage.

mod support;

// These grammar tests never reach provider capabilities.
fn command_router()
-> omnia_guest::api::command::Router<support::Inert, emery_transport::command::Globals> {
    support::router()
}

// An HTTP operation surface requires an authenticated ingress design.
#[tokio::test]
async fn adr_0002_http_refusal() {
    use omnia_guest::http::{Method, Request, StatusCode};
    use tower::ServiceExt as _;

    // Derivation from inventory prevents new verbs gaining HTTP twins,
    // the spec shelf route included.
    let command = command_router();
    for route in command.inventory() {
        let path = format!("/{}", route.selector().path().join("/"));
        for method in [Method::GET, Method::POST] {
            let request = Request::builder()
                .method(method.clone())
                .uri(&path)
                .body(omnia_guest::axum::body::Body::empty())
                .expect("build request");
            let response = emery_transport::http::listener(support::Inert::default())
                .oneshot(request)
                .await
                .expect("the listener serves the request");
            assert_eq!(response.status(), StatusCode::NOT_FOUND, "{method} {path} must refuse");
        }
    }
}

// Widening the route budget requires an ADR.
#[tokio::test]
async fn adr_0008_route_budget() {
    let router = command_router();

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

#[tokio::test]
async fn specify_without_sources() {
    let provider = support::Inert::default();
    let storage = std::sync::Arc::clone(&provider.storage);
    let router = support::router_over(provider);

    let response = router.execute(["emery", "specify"]).await;
    assert_eq!(response.exit, 2);
    let stderr = String::from_utf8_lossy(&response.stderr);
    assert!(stderr.contains("at least one source"), "{stderr}");

    let json = router.execute(["emery", "--format", "json", "specify"]).await;
    assert_eq!(json.exit, 2);
    let stderr = String::from_utf8(json.stderr).expect("stderr utf-8");
    let envelope: serde_json::Value = serde_json::from_str(&stderr).expect("one JSON envelope");
    assert_eq!(envelope["error"], "specify-source-required");
    assert_eq!(envelope["exit-code"], 2);

    assert!(storage.is_empty(), "a refused run writes nothing");
}

// `--sources` carries the whole binding list; mixing refuses typed.
#[tokio::test]
async fn specify_mixed_sources_refused() {
    let provider = support::Inert::default();
    let storage = std::sync::Arc::clone(&provider.storage);
    let router = support::router_over(provider);

    for argv in [
        &["emery", "specify", "docs", "--sources", "sources.toml"][..],
        &["emery", "specify", "--value", "intent=text", "--sources", "sources.toml"][..],
    ] {
        let mut json = vec!["emery", "--format", "json"];
        json.extend(argv.iter().skip(1));
        let response = router.execute(json).await;
        assert_eq!(response.exit, 2, "{argv:?}");
        let stderr = String::from_utf8(response.stderr).expect("stderr utf-8");
        let envelope: serde_json::Value = serde_json::from_str(&stderr).expect("one JSON envelope");
        assert_eq!(envelope["error"], "argument", "{argv:?}");
        assert_eq!(envelope["exit-code"], 2, "{argv:?}");
    }

    assert!(storage.is_empty(), "a refused run writes nothing");
}

// The read verb fails typed before any generation is committed.
#[tokio::test]
async fn show_without_generation() {
    let provider = support::Inert::default();
    let storage = std::sync::Arc::clone(&provider.storage);
    let router = support::router_over(provider);

    let response = router.execute(["emery", "show", "spec"]).await;
    assert_eq!(response.exit, 1);
    let stderr = String::from_utf8_lossy(&response.stderr);
    assert!(stderr.contains("spec-not-generated"), "{stderr}");

    let json = router.execute(["emery", "--format", "json", "show", "design"]).await;
    assert_eq!(json.exit, 1);
    let stderr = String::from_utf8(json.stderr).expect("stderr utf-8");
    let envelope: serde_json::Value = serde_json::from_str(&stderr).expect("one JSON envelope");
    assert_eq!(envelope["error"], "spec-not-generated");
    assert_eq!(envelope["exit-code"], 1);

    assert!(storage.is_empty(), "a refused read writes nothing");
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
    // Adapters version independently, so the binary reports its own SemVer.
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
    let expected = router.execute(["emery", "specify", "--no-such-flag"]).await;
    let forwarded = router.execute(["emery:engine@0.1.0", "specify", "--no-such-flag"]).await;

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

#[tokio::test]
async fn native_response_contract() {
    for case in cases() {
        // A fresh store keeps `specify` sourceless and `show` without a generation.
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
