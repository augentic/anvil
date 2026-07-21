//! Launcher-facing selector projection over the shared grammar, plus
//! the grammar-coverage guard: every route in the router inventory
//! must carry a [`selectors::SCOPE_ROUTES`] closure classification
//! (and selector-bearing routes must fold through
//! [`selectors::SELECTOR_ROUTES`]), so a new verb cannot land without
//! deciding what the launcher deploys for it.

use std::collections::BTreeSet;

use mock::model::Harness;
use native::{DynModel, Provider, ReferenceMode};
use omnia_guest::api::invoke::Invoker;
use transport::command::Format;
use transport::command::selectors::{self, ClosureScope, CommandSelectors, Projection, from_argv};

fn argv(args: &[&str]) -> Vec<String> {
    args.iter().map(ToString::to_string).collect()
}

fn forwarded(args: &[&str]) -> CommandSelectors {
    match from_argv(&argv(args)) {
        Projection::Forward(selectors) => selectors,
        Projection::Rejected { rendered } => panic!("argv {args:?} rejected: {rendered}"),
    }
}

fn rejected(args: &[&str]) -> String {
    match from_argv(&argv(args)) {
        Projection::Rejected { rendered } => rendered,
        Projection::Forward(selectors) => panic!("argv {args:?} forwarded: {selectors:?}"),
    }
}

#[test]
fn init_adapter() {
    let selectors = forwarded(&["init", "./mock.wasm", "--name", "demo"]);
    assert_eq!(selectors.targets, ["./mock.wasm"]);
    assert!(selectors.sources.is_empty());
}

#[test]
fn init_workspace() {
    let selectors = forwarded(&["init", "--workspace"]);
    assert!(selectors.targets.is_empty());
    assert!(selectors.sources.is_empty());
}

#[test]
fn source_resolve() {
    let selectors = forwarded(&["source", "resolve", "typescript"]);
    assert_eq!(selectors.sources, ["typescript"]);
    assert!(selectors.targets.is_empty());
}

#[test]
fn target_resolve() {
    let selectors = forwarded(&["target", "resolve", "specify:omnia@1.0.0"]);
    assert_eq!(selectors.targets, ["specify:omnia@1.0.0"]);
    assert!(selectors.sources.is_empty());
}

#[test]
fn plan_author_bindings_and_intent_sugar() {
    let selectors = forwarded(&[
        "plan",
        "author",
        "change",
        "--source",
        "docs=documentation:./docs",
        "--source",
        "main=mock:value:The greeting service.",
        "--intent",
        "One brief.",
    ]);
    assert_eq!(selectors.sources, ["documentation", "mock", "intent"]);
    assert!(selectors.targets.is_empty());
}

#[test]
fn format_global() {
    assert_eq!(forwarded(&["registry", "validate"]).format, Format::Text);
    assert_eq!(forwarded(&["--format", "json", "registry", "validate"]).format, Format::Json);
}

#[test]
fn selector_free_route() {
    let selectors = forwarded(&["plan", "status"]);
    assert!(selectors.sources.is_empty());
    assert!(selectors.targets.is_empty());
}

#[test]
fn adapter_add_engine_only() {
    // The cache copy is deterministic engine work: the component path
    // is the copy's input, never a deployment requirement — but the
    // launcher must see it to perform the seed host-side.
    let selectors = forwarded(&["adapter", "add", "./demo.wasm"]);
    assert!(selectors.sources.is_empty());
    assert!(selectors.targets.is_empty());
    assert_eq!(selectors.scope, ClosureScope::ENGINE);
    let seed = selectors.seed.expect("adapter add projects its seed request");
    assert_eq!(seed.component, std::path::PathBuf::from("./demo.wasm"));
    assert_eq!(seed.project_dir, Some(std::path::PathBuf::from(".")));
}

#[test]
fn adapter_add_project_dir() {
    let selectors = forwarded(&["adapter", "add", "demo.wasm", "--project-dir", "/tmp/proj"]);
    let seed = selectors.seed.expect("seed request");
    assert_eq!(seed.project_dir, Some(std::path::PathBuf::from("/tmp/proj")));
}

#[test]
fn selector_free_routes_project_no_seed() {
    assert_eq!(forwarded(&["plan", "status"]).seed, None);
    assert_eq!(forwarded(&["init", "./mock.wasm"]).seed, None);
}

#[test]
fn help_and_version_forward_empty() {
    for display in [&["--help"][..], &["plan", "--help"][..], &["--version"][..]] {
        let selectors = forwarded(display);
        assert!(selectors.sources.is_empty(), "{display:?}");
        assert!(selectors.targets.is_empty(), "{display:?}");
        assert_eq!(selectors.scope, ClosureScope::ENGINE, "{display:?}");
    }
}

#[test]
fn scope_projection() {
    // Read-only projections stay engine-only; the closure joins no
    // state-derived adapters for them.
    for engine_only in [&["plan", "status"][..], &["journal", "show"][..], &["slice", "list"][..]] {
        assert_eq!(forwarded(engine_only).scope, ClosureScope::ENGINE, "{engine_only:?}");
    }
    assert_eq!(forwarded(&["slice", "build", "s1"]).scope, ClosureScope::PROJECT_TARGET);
    assert_eq!(forwarded(&["source", "survey", "main"]).scope, ClosureScope::PLAN_SOURCES);
    assert_eq!(forwarded(&["slice", "refine", "s1"]).scope, ClosureScope::REFINE);
    assert_eq!(forwarded(&["plan", "validate"]).scope, ClosureScope::SLOT_TARGETS);
    assert_eq!(forwarded(&["plan", "execute"]).scope, ClosureScope::FULL);
}

#[test]
fn grammar_failures_reject() {
    let unknown = rejected(&["frobnicate"]);
    assert!(unknown.contains("unrecognized subcommand"), "{unknown}");

    let missing = rejected(&["plan", "transition"]);
    assert!(missing.contains("Usage: specify plan transition"), "{missing}");

    let malformed = rejected(&["plan", "author", "change", "--source", "no-equals"]);
    assert!(malformed.contains("--source"), "{malformed}");
}

/// Every router route must be classified in
/// [`selectors::SCOPE_ROUTES`]: which state-derived closure legs the
/// launcher joins for the verb. Selector-bearing routes must
/// additionally fold through [`selectors::SELECTOR_ROUTES`]. A new
/// verb fails here until both decisions are made — the RFC-70
/// closure-superset invariant guard.
#[test]
fn grammar_coverage() {
    let locations = project::handler::Locations::explicit(
        std::path::PathBuf::from("store"),
        project::handler::CachePlacement::Parent(std::path::PathBuf::from("project-cache")),
    );
    let provider = Provider::new(
        project::handler::ExecutionPaths::new(".", locations),
        DynModel::new(Harness::answering(Vec::<String>::new())),
        mock::catalog(),
        ReferenceMode::Offline,
    );
    let router = transport::command::router(Invoker::new("specify", provider)).expect("router");
    let inventory: BTreeSet<Vec<String>> =
        router.inventory().iter().map(|route| route.selector().path().to_vec()).collect();

    let scoped: BTreeSet<Vec<String>> = selectors::SCOPE_ROUTES
        .iter()
        .map(|(path, _)| path.iter().map(ToString::to_string).collect())
        .collect();

    let unclassified: Vec<_> = inventory.difference(&scoped).collect();
    assert!(
        unclassified.is_empty(),
        "unclassified routes {unclassified:?}: add each to selectors::SCOPE_ROUTES with the \
         closure legs its operation can dispatch, and — if it carries adapter selectors in argv \
         — extend selectors::from_argv + SELECTOR_ROUTES"
    );
    let stale: Vec<_> = scoped.difference(&inventory).collect();
    assert!(stale.is_empty(), "classified routes no longer in the grammar: {stale:?}");

    let selector_bearing: BTreeSet<Vec<String>> = selectors::SELECTOR_ROUTES
        .iter()
        .map(|path| path.iter().map(ToString::to_string).collect())
        .collect();
    let unrouted: Vec<_> = selector_bearing.difference(&inventory).collect();
    assert!(unrouted.is_empty(), "selector routes no longer in the grammar: {unrouted:?}");
}
