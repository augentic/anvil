//! Typed command grammar, conversion, and HTTP parity coverage.

use std::any::TypeId;
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

use native::{DynModel, Provider, ReferenceMode};
use omnia_guest::api::invoke::Invoker;
use omnia_testkit::model::Harness;
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
    transport::command::router(Invoker::new("emery", provider(root))).expect("router")
}

#[test]
fn http_parity() {
    let command = command_router(".");
    let command_types: BTreeSet<TypeId> = command
        .inventory()
        .iter()
        .filter_map(omnia_guest::api::command::RouteInfo::operation)
        .collect();
    let http_types: BTreeSet<TypeId> =
        transport::http::router(Invoker::new("emery", provider(".")))
            .inventory()
            .iter()
            .map(omnia_guest::api::http::RouteInfo::operation)
            .collect();
    let transport_only: BTreeSet<Vec<String>> = command
        .inventory()
        .iter()
        .filter(|route| route.operation().is_none_or(|operation| !http_types.contains(&operation)))
        .map(|route| route.selector().path().to_vec())
        .collect();
    let expected: BTreeSet<Vec<String>> = std::iter::once(&["completions"][..])
        .map(|path| path.iter().map(|part| (*part).to_string()).collect())
        .collect();

    assert_eq!(transport_only, expected);
    assert_eq!(http_types.difference(&command_types).count(), 0);
    assert_eq!(command_types.difference(&http_types).count(), 0);
    assert_eq!(http_types.len(), 29);
}

#[tokio::test]
async fn globals_and_completions() {
    let router = command_router(".");

    let help = router.execute(["emery", "plan", "amend", "--help"]).await;
    assert_eq!(help.exit, 0);
    assert!(String::from_utf8_lossy(&help.stdout).contains("--allow-composition-replace"));

    let completions = router.execute(["emery", "completions", "zsh"]).await;
    assert_eq!(completions.exit, 0);
    assert!(!completions.stdout.is_empty());
    let completion_help = router.execute(["emery", "completions", "--help"]).await;
    let completion_help = String::from_utf8_lossy(&completion_help.stdout);
    assert!(completion_help.contains("Pipe into your shell's completion directory"));
    assert!(completion_help.contains("output tracks the live clap surface"));

    let invalid = router.execute(["emery", "--format", "json", "plan", "drop"]).await;
    assert_eq!(invalid.exit, 2);
}

#[tokio::test]
async fn detailed_help() {
    let router = command_router(".");

    let route = router.execute(["emery", "plan", "status", "--help"]).await;
    assert_eq!(route.exit, 0);
    let route = String::from_utf8_lossy(&route.stdout);
    assert!(route.contains("Read-only projection of the plan's execution state"));
    assert!(route.contains("Stop reasons (`refine-failed`"));

    let namespace = router.execute(["emery", "source", "--help"]).await;
    assert_eq!(namespace.exit, 0);
    let namespace = String::from_utf8_lossy(&namespace.stdout);
    assert!(namespace.contains("Source adapter operations (workflow contract)"));
    assert!(namespace.contains("provide `extract` + `survey` capabilities"));

    let nested = router.execute(["emery", "slice", "model", "--help"]).await;
    assert_eq!(nested.exit, 0);
    assert!(
        String::from_utf8_lossy(&nested.stdout)
            .contains("Read-only viewer over a slice's `model.yaml`")
    );

    for removed in [
        &["emery", "adapters", "sync"][..],
        &["emery", "plugins", "doctor"][..],
        &["emery", "plugins", "refresh"][..],
        &["emery", "upgrade"][..],
        &["emery", "workspace", "prepare"][..],
        &["emery", "workspace", "push"][..],
        &["emery", "workspace", "sync"][..],
        // RFC-88 D4 — the registry/workspace topology feature is removed.
        &["emery", "registry", "add"][..],
        &["emery", "registry", "validate"][..],
        &["emery", "registry", "remove"][..],
        &["emery", "init", "--workspace"][..],
        // RFC-86 D14 / D6 — never shipped. (`plan refine` is a real
        // verb since RFC-91 — asserted present below, not here.)
        &["emery", "plan", "approve"][..],
        // Plan-centric surface cut — the slice-loop breakout verbs and
        // plan advance/undo are gone; `plan execute` owns the phases.
        &["emery", "slice", "refine"][..],
        &["emery", "slice", "build"][..],
        &["emery", "slice", "merge"][..],
        &["emery", "slice", "drop"][..],
        &["emery", "plan", "advance"][..],
        &["emery", "plan", "undo"][..],
    ] {
        assert_eq!(router.execute(removed.iter().copied()).await.exit, 2, "{removed:?}");
    }

    let plan_help = router.execute(["emery", "plan", "--help"]).await;
    assert_eq!(plan_help.exit, 0);
    let plan_help = String::from_utf8_lossy(&plan_help.stdout);
    assert!(!plan_help.contains("approve"), "no plan approve subcommand: {plan_help}");
    // `plan refine` is the RFC-91 refinement drain — a real subcommand.
    assert!(
        plan_help.lines().any(|line| line.trim_start().starts_with("refine")),
        "plan help must list refine: {plan_help}"
    );
    assert!(
        !plan_help.lines().any(|line| {
            let trimmed = line.trim_start();
            trimmed.starts_with("approve")
                || trimmed.starts_with("advance")
                || trimmed.starts_with("undo")
        }),
        "plan help must not list approve/advance/undo: {plan_help}"
    );
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
    let expected = router.execute(["emery", "plan", "drop"]).await;
    let forwarded = router.execute(["emery:engine@0.1.0", "plan", "drop"]).await;

    assert_eq!(expected.exit, 2);
    assert_eq!(forwarded.exit, expected.exit);
    assert_eq!(forwarded.stderr, expected.stderr);
    let stderr = String::from_utf8_lossy(&forwarded.stderr);
    assert!(stderr.contains("Usage: emery plan drop"));
    assert!(!stderr.contains("emery:engine@0.1.0"));
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

const fn cases() -> [Case; 10] {
    [
        Case {
            name: "debt empty baseline",
            argv: &["emery", "debt"],
            fixture: Fixture::Project,
            exit: 0,
            stdout: "baseline debt: none",
            stderr: "",
            json_channels: false,
        },
        Case {
            name: "help",
            argv: &["emery", "--help"],
            fixture: Fixture::Project,
            exit: 0,
            stdout: "Usage: emery [OPTIONS] <COMMAND>",
            stderr: "",
            json_channels: false,
        },
        Case {
            name: "version",
            argv: &["emery", "--version"],
            fixture: Fixture::Project,
            exit: 0,
            stdout: concat!("emery ", env!("CARGO_PKG_VERSION")),
            stderr: "",
            json_channels: false,
        },
        Case {
            name: "completions",
            argv: &["emery", "completions", "zsh"],
            fixture: Fixture::Project,
            exit: 0,
            stdout: "_emery",
            stderr: "",
            json_channels: false,
        },
        Case {
            name: "text",
            argv: &["emery", "journal", "show"],
            fixture: Fixture::Project,
            exit: 0,
            stdout: "no events",
            stderr: "",
            json_channels: false,
        },
        Case {
            name: "json",
            argv: &["emery", "--format", "json", "journal", "show"],
            fixture: Fixture::Project,
            exit: 0,
            stdout: "\"count\": 0",
            stderr: "",
            json_channels: true,
        },
        Case {
            name: "generic failure",
            argv: &["emery", "plan", "validate"],
            fixture: Fixture::Project,
            exit: 1,
            stdout: "",
            stderr: "plan.yaml",
            json_channels: false,
        },
        Case {
            name: "usage",
            argv: &["emery", "plan", "drop"],
            fixture: Fixture::Project,
            exit: 2,
            stdout: "",
            stderr: "Usage: emery plan drop",
            json_channels: false,
        },
        Case {
            name: "validation report",
            argv: &["emery", "--format", "json", "plan", "validate"],
            fixture: Fixture::Cycle,
            exit: 2,
            stdout: "cycle-in-depends-on",
            stderr: "\"exit-code\": 2",
            json_channels: true,
        },
        Case {
            name: "version floor",
            argv: &["emery", "--format", "json", "journal", "show"],
            fixture: Fixture::TooNew,
            exit: 3,
            stdout: "",
            stderr: "emery-version-too-old",
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
    let emery = project.path().join(".emery");
    fs::create_dir(&emery).expect("create .emery");
    let version = match fixture {
        Fixture::TooNew => "999.0.0",
        Fixture::Project | Fixture::Cycle => env!("CARGO_PKG_VERSION"),
    };
    fs::write(
        emery.join("project.yaml"),
        format!("name: router-parity\nadapter: omnia\nemery: {version}\nrules: {{}}\n"),
    )
    .expect("write project config");
    if matches!(fixture, Fixture::Cycle) {
        fs::write(
            project.path().join("plan.yaml"),
            "name: cycle\nsources: {}\nslices:\n  - name: first\n    depends-on: [second]\n  - name: second\n    depends-on: [first]\n",
        )
        .expect("write cyclic plan");
    }
    project
}
