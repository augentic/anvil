//! Typed command grammar, conversion, and HTTP parity coverage.

use std::any::TypeId;
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use mock::model::Harness;
use native::{DynModel, Provider, ReferenceMode};
use omnia_guest::api::invoke::Invoker;
use tempfile::TempDir;

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
        ReferenceMode::Offline,
    )
}

fn command_router(
    root: impl Into<PathBuf>,
) -> omnia_guest::api::command::Router<Provider, transport::command::Globals> {
    transport::command::router(Invoker::new("specify", provider(root))).expect("router")
}

#[test]
fn http_parity() {
    let command = command_router(".");
    let command_types: BTreeSet<TypeId> = command
        .inventory()
        .iter()
        .filter_map(omnia_guest::api::command::RouteInfo::operation_type_id)
        .collect();
    let http_types: BTreeSet<TypeId> =
        transport::http::router(Invoker::new("specify", provider(".")))
            .inventory()
            .iter()
            .map(omnia_guest::api::http::RouteInfo::operation)
            .collect();
    let transport_only: BTreeSet<Vec<String>> = command
        .inventory()
        .iter()
        .filter(|route| {
            route.operation_type_id().is_none_or(|operation| !http_types.contains(&operation))
        })
        .map(|route| route.selector().path().to_vec())
        .collect();
    let expected: BTreeSet<Vec<String>> = std::iter::once(&["completions"][..])
        .map(|path| path.iter().map(|part| (*part).to_string()).collect())
        .collect();

    assert_eq!(transport_only, expected);
    assert_eq!(http_types.difference(&command_types).count(), 0);
    assert_eq!(command_types.difference(&http_types).count(), 0);
    assert_eq!(http_types.len(), 32);
}

#[tokio::test]
async fn globals_and_completions() {
    let router = command_router(".");

    let help = router.execute(["specify", "slice", "merge", "run", "--help"]).await;
    assert_eq!(help.exit, 0);
    assert!(String::from_utf8_lossy(&help.stdout).contains("--allow-composition-replace"));

    let completions = router.execute(["specify", "completions", "zsh"]).await;
    assert_eq!(completions.exit, 0);
    assert!(!completions.stdout.is_empty());
    let completion_help = router.execute(["specify", "completions", "--help"]).await;
    let completion_help = String::from_utf8_lossy(&completion_help.stdout);
    assert!(completion_help.contains("Pipe into your shell's completion directory"));
    assert!(completion_help.contains("output tracks the live clap surface"));

    let invalid = router.execute(["specify", "--format", "json", "plan", "transition"]).await;
    assert_eq!(invalid.exit, 2);
}

#[tokio::test]
async fn detailed_help() {
    let router = command_router(".");

    let route = router.execute(["specify", "plan", "status", "--help"]).await;
    assert_eq!(route.exit, 0);
    let route = String::from_utf8_lossy(&route.stdout);
    assert!(route.contains("Read-only projection of the plan's execution state"));
    assert!(route.contains("Stop reasons (`plan-not-approved`"));

    let namespace = router.execute(["specify", "source", "--help"]).await;
    assert_eq!(namespace.exit, 0);
    let namespace = String::from_utf8_lossy(&namespace.stdout);
    assert!(namespace.contains("Source adapter operations (workflow contract)"));
    assert!(namespace.contains("provide `extract` + `survey` capabilities"));

    let nested = router.execute(["specify", "slice", "model", "--help"]).await;
    assert_eq!(nested.exit, 0);
    assert!(
        String::from_utf8_lossy(&nested.stdout)
            .contains("Read-only viewer over a slice's `model.yaml`")
    );

    for removed in [
        &["specify", "adapters", "sync"][..],
        &["specify", "plugins", "doctor"][..],
        &["specify", "plugins", "refresh"][..],
        &["specify", "upgrade"][..],
        &["specify", "workspace", "prepare"][..],
        &["specify", "workspace", "push"][..],
        &["specify", "workspace", "sync"][..],
    ] {
        assert_eq!(router.execute(removed.iter().copied()).await.exit, 2, "{removed:?}");
    }
}

#[tokio::test]
async fn argv_zero_replaced() {
    let router = command_router(".");
    let expected = router.execute(["specify", "plan", "transition"]).await;
    let forwarded = router.execute(["specify:engine@0.1.0", "plan", "transition"]).await;

    assert_eq!(expected.exit, 2);
    assert_eq!(forwarded.exit, expected.exit);
    assert_eq!(forwarded.stderr, expected.stderr);
    let stderr = String::from_utf8_lossy(&forwarded.stderr);
    assert!(stderr.contains("Usage: specify plan transition"));
    assert!(!stderr.contains("specify:engine@0.1.0"));
}

#[derive(Clone, Copy)]
enum Fixture {
    Project,
    Cycle,
    TooNew,
}

struct Case {
    name: &'static str,
    argv: &'static [&'static str],
    fixture: Fixture,
    exit: u8,
    stdout: &'static str,
    stderr: &'static str,
    json_channels: bool,
}

const fn cases() -> [Case; 9] {
    [
        Case {
            name: "help",
            argv: &["specify", "--help"],
            fixture: Fixture::Project,
            exit: 0,
            stdout: "Usage: specify [OPTIONS] <COMMAND>",
            stderr: "",
            json_channels: false,
        },
        Case {
            name: "version",
            argv: &["specify", "--version"],
            fixture: Fixture::Project,
            exit: 0,
            stdout: concat!("specify ", env!("CARGO_PKG_VERSION")),
            stderr: "",
            json_channels: false,
        },
        Case {
            name: "completions",
            argv: &["specify", "completions", "zsh"],
            fixture: Fixture::Project,
            exit: 0,
            stdout: "_specify",
            stderr: "",
            json_channels: false,
        },
        Case {
            name: "text",
            argv: &["specify", "registry", "validate"],
            fixture: Fixture::Project,
            exit: 0,
            stdout: "no registry declared at registry.yaml",
            stderr: "",
            json_channels: false,
        },
        Case {
            name: "json",
            argv: &["specify", "--format", "json", "registry", "validate"],
            fixture: Fixture::Project,
            exit: 0,
            stdout: "\"registry\": null",
            stderr: "",
            json_channels: true,
        },
        Case {
            name: "generic failure",
            argv: &["specify", "plan", "validate"],
            fixture: Fixture::Project,
            exit: 1,
            stdout: "",
            stderr: "plan.yaml",
            json_channels: false,
        },
        Case {
            name: "usage",
            argv: &["specify", "plan", "transition"],
            fixture: Fixture::Project,
            exit: 2,
            stdout: "",
            stderr: "Usage: specify plan transition",
            json_channels: false,
        },
        Case {
            name: "validation report",
            argv: &["specify", "--format", "json", "plan", "validate"],
            fixture: Fixture::Cycle,
            exit: 2,
            stdout: "cycle-in-depends-on",
            stderr: "\"exit-code\": 2",
            json_channels: true,
        },
        Case {
            name: "version floor",
            argv: &["specify", "--format", "json", "registry", "validate"],
            fixture: Fixture::TooNew,
            exit: 3,
            stdout: "",
            stderr: "specify-version-too-old",
            json_channels: true,
        },
    ]
}

#[tokio::test]
async fn native_response_contract() {
    for case in cases() {
        let project = project(case.fixture);
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

fn project(fixture: Fixture) -> TempDir {
    let project = tempfile::tempdir().expect("tempdir");
    let specify = project.path().join(".specify");
    fs::create_dir(&specify).expect("create .specify");
    let version = match fixture {
        Fixture::TooNew => "999.0.0",
        Fixture::Project | Fixture::Cycle => env!("CARGO_PKG_VERSION"),
    };
    fs::write(
        specify.join("project.yaml"),
        format!("name: router-parity\nadapter: omnia\nspecify: {version}\nrules: {{}}\n"),
    )
    .expect("write project config");
    if matches!(fixture, Fixture::Cycle) {
        fs::write(
            project.path().join("plan.yaml"),
            "name: cycle\nsources: {}\nslices:\n  - name: first\n    status: pending\n    depends-on: [second]\n  - name: second\n    status: pending\n    depends-on: [first]\n",
        )
        .expect("write cyclic plan");
    }
    project
}
