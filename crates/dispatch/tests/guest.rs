//! In-process coverage of the guest-facing dispatch surface: the
//! shared-grammar [`parse`] entry point's exit-code contract and the
//! [`route`] table's three-way split (in-process pure verbs,
//! shim-dispatched orchestrations, refused native-only verbs).
//!
//! The full per-verb behaviour (envelopes, side effects, golden files)
//! stays covered by the binary's subprocess suite in `tests/`;
//! these tests pin the seam the workflow guest depends on.

use specify_dispatch::guest::{Orchestration, Route, Verb, parse, route};
use specify_dispatch::output::Exit;

/// Parse one argv line (program name included) through the shared
/// grammar, panicking on parse failure.
fn parse_ok(argv: &[&str]) -> specify_dispatch::cli::Cli {
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
    assert_eq!(exit.code(), 2, "clap usage errors exit 2, matching native");
}

#[test]
fn survey_routes_to_orchestrator() {
    let cli = parse_ok(&["specify", "source", "survey", "typescript", "--plan", "demo"]);
    let Route::Orchestrate(Orchestration { verb, .. }) = route(cli) else {
        panic!("source survey must route to the shim's orchestrator dispatch");
    };
    assert_eq!(
        verb,
        Verb::Survey {
            source: "typescript".to_string(),
            plan: Some("demo".to_string()),
        }
    );
}

#[test]
fn extract_routes_to_orchestrator() {
    let cli =
        parse_ok(&["specify", "source", "extract", "typescript", "billing", "--slice", "billing"]);
    let Route::Orchestrate(Orchestration { verb, .. }) = route(cli) else {
        panic!("source extract must route to the shim's orchestrator dispatch");
    };
    assert_eq!(
        verb,
        Verb::Extract {
            source: "typescript".to_string(),
            lead: "billing".to_string(),
            slice: "billing".to_string(),
        }
    );
}

#[test]
fn build_routes_to_orchestrator() {
    let cli = parse_ok(&["specify", "slice", "build", "billing"]);
    let Route::Orchestrate(Orchestration { verb, .. }) = route(cli) else {
        panic!("slice build must route to the shim's orchestrator dispatch");
    };
    assert_eq!(
        verb,
        Verb::Build {
            slice: "billing".to_string(),
        }
    );
}

#[test]
fn merge_run_routes_to_orchestrator() {
    let cli = parse_ok(&["specify", "slice", "merge", "run", "billing"]);
    let Route::Orchestrate(Orchestration { verb, .. }) = route(cli) else {
        panic!("slice merge run must route to the shim's orchestrator dispatch");
    };
    assert_eq!(
        verb,
        Verb::Merge {
            slice: "billing".to_string(),
            allow_composition_replace: false,
        }
    );
}

#[test]
fn plan_execute_routes_to_orchestrator() {
    // The drained execute loop is guest-only argv: in-guest it routes
    // to the shim's orchestrator dispatch (natively the shared verb
    // table refuses it — covered by the binary suite).
    let cli = parse_ok(&["specify", "plan", "execute"]);
    let Route::Orchestrate(Orchestration { verb, .. }) = route(cli) else {
        panic!("plan execute must route to the shim's orchestrator dispatch");
    };
    assert_eq!(verb, Verb::Execute);
}

#[test]
fn plan_author_routes_with_bindings() {
    // `plan author` is guest-only argv: the route desugars
    // `--source` / `--intent` into the structured binding map the
    // orchestrator hands `Plan::init`.
    let cli = parse_ok(&[
        "specify",
        "plan",
        "author",
        "account-revamp",
        "--source",
        "docs=documentation:./design-notes",
        "--intent",
        "Refresh registration.",
    ]);
    let Route::Orchestrate(Orchestration { verb, .. }) = route(cli) else {
        panic!("plan author must route to the shim's orchestrator dispatch");
    };
    let Verb::Author { name, sources } = verb else {
        panic!("expected the author verb, got {verb:?}");
    };
    assert_eq!(name, "account-revamp");
    let keys: Vec<&str> = sources.keys().map(String::as_str).collect();
    assert_eq!(keys, ["docs", "intent"]);
    assert_eq!(sources["docs"].adapter, "documentation");
    assert_eq!(sources["docs"].path.as_deref(), Some("./design-notes"));
    assert_eq!(sources["intent"].adapter, "intent");
    assert_eq!(sources["intent"].value.as_deref(), Some("Refresh registration."));
}

#[test]
fn plan_author_duplicate_binding_refused() {
    // `--intent` desugars to the intent binding before the
    // duplicate-key gate, so pairing it with an explicit
    // `--source intent=...` refuses in-route (the `plan create`
    // semantics verbatim).
    let cli = parse_ok(&[
        "specify",
        "plan",
        "author",
        "account-revamp",
        "--source",
        "intent=intent:value:one",
        "--intent",
        "two",
    ]);
    let Route::Handled(exit) = route(cli) else {
        panic!("a duplicate binding must be refused in-process, not orchestrated");
    };
    assert_eq!(exit, Exit::GenericFailure, "plan-source-duplicate-key maps to the generic exit");
}

#[test]
fn slice_refine_routes_to_orchestrator() {
    // The `/spec:refine` breakout is guest-only argv (native
    // parity gap 2); natively the shared verb table refuses it.
    let cli = parse_ok(&["specify", "slice", "refine", "billing"]);
    let Route::Orchestrate(Orchestration { verb, .. }) = route(cli) else {
        panic!("slice refine must route to the shim's orchestrator dispatch");
    };
    assert_eq!(
        verb,
        Verb::Refine {
            slice: "billing".to_string(),
        }
    );
}

#[test]
fn global_flags_thread_to_orchestration() {
    // `--plan-dir` is guarded to the project root (the guest's `"."`
    // mount preopen), so thread the CWD through and assert it survives
    // onto the orchestration descriptor.
    let cwd = std::env::current_dir().expect("cwd");
    let plan_dir = cwd.to_str().expect("utf-8 cwd");
    let cli = parse_ok(&[
        "specify",
        "--format",
        "json",
        "--plan-dir",
        plan_dir,
        "slice",
        "build",
        "billing",
    ]);
    let Route::Orchestrate(orchestration) = route(cli) else {
        panic!("slice build must route to the shim's orchestrator dispatch");
    };
    assert_eq!(orchestration.plan_dir.as_deref(), Some(cwd.as_path()));
}

#[test]
fn plan_dir_outside_project_root_refused() {
    // Plan artifacts anchor at the `"."` preopen, so any other plan
    // root would be silently ignored; the route refuses it instead.
    let cli = parse_ok(&["specify", "--plan-dir", "/tmp/plan-root", "slice", "build", "billing"]);
    let Route::Handled(exit) = route(cli) else {
        panic!("a foreign --plan-dir must be refused in-process, not orchestrated");
    };
    assert_eq!(exit, Exit::ArgumentError, "foreign --plan-dir refuses with the argument exit");
}

#[test]
fn native_only_verbs_refused_exit_two() {
    // One verb per refused family; each renders the native
    // argument-error envelope (wire code `argument`) and exits 2.
    for argv in [
        vec!["specify", "init", "omnia"],
        vec!["specify", "workspace", "sync"],
        vec!["specify", "upgrade"],
    ] {
        let cli = parse_ok(&argv);
        let Route::Handled(exit) = route(cli) else {
            panic!("{argv:?} must be refused in-process, not orchestrated");
        };
        assert_eq!(exit, Exit::ArgumentError, "{argv:?} must refuse with the argument error code");
    }
    // `lint framework` moved in-guest with the native provisioning
    // surface's retirement: it parses on the shared grammar (hidden
    // from help) rather than being refused. Parse-only here — routing
    // it would walk the framework root for real.
    parse_ok(&["specify", "lint", "framework"]);
}

#[test]
fn route_runs_pure_verbs_in_process() {
    // Outside any project, a pure workflow verb still dispatches
    // in-process and surfaces the native `not-initialized` failure —
    // proof the handler ran rather than being refused at the routing
    // table. Safe to re-anchor the CWD: nextest runs each test in its
    // own process.
    let scratch = tempfile::tempdir().expect("scratch dir");
    std::env::set_current_dir(scratch.path()).expect("enter scratch dir");
    let cli = parse_ok(&["specify", "plan", "status"]);
    let Route::Handled(exit) = route(cli) else {
        panic!("plan status is a pure workflow verb and must run in-process");
    };
    assert_eq!(exit, Exit::GenericFailure, "not-initialized maps to the generic failure exit");
}
