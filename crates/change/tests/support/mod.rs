//! Suite-local source bindings and plan builders over `change` wire
//! types.
//!
//! The builders return `change` types, so they live with the owning
//! suites rather than in the mock crate, keeping `mock` free of
//! workflow-crate dependencies.

#![allow(dead_code, reason = "each test binary uses a subset of the shared support surface")]

use change::plan::wire::SourceAssign;
use mock::session::Session;
use project::handler::Anchor as _;
use serde_json::json;

/// Drive the refine phase for one plan entry the way the execute
/// loop's advance step does — the standalone `slice refine` verb is
/// retired, so suites reach the orchestration directly.
///
/// # Errors
///
/// Propagates the orchestration's failures.
///
/// # Panics
///
/// Panics when `plan.yaml` is absent or carries no entry named
/// `slice`.
pub async fn refine(
    session: &Session, slice: &str,
) -> Result<slice::orchestrate::RefineOutcome, error::Error> {
    let provider = session.provider();
    let caps = slice::orchestrate::Capabilities::provider(provider);
    let paths = provider.paths();
    let layout = project::config::Layout::new(paths.project_root());
    let plan = project::plan::Plan::load(&layout.plan_path())?;
    let entry = plan
        .entries
        .iter()
        .find(|entry| entry.name == slice)
        .unwrap_or_else(|| panic!("plan entry `{slice}` missing"));
    let target = project::target_policy::resumed(layout, slice)
        .or_else(|_| project::target_policy::fresh(provider, paths, entry, slice, "refining"))?;
    slice::orchestrate::refine(caps, paths, jiff::Timestamp::now(), slice, &target).await
}

/// Claim the next eligible plan entry — the execute loop's advance
/// step, reached directly now that the standalone verb is retired.
///
/// # Panics
///
/// Panics when config load or the advance kernel fails.
pub fn advance(session: &Session) -> project::plan::AdvanceBody {
    let provider = session.provider();
    let paths = provider.paths();
    let layout = project::config::Layout::new(paths.project_root());
    let config = project::config::ProjectConfig::load(layout.project_dir()).expect("config loads");
    project::plan::advance_next(provider, paths, jiff::Timestamp::now(), &config)
        .expect("advance claims")
}

/// Drive the build phase for one slice the way the execute loop does.
///
/// # Errors
///
/// Propagates config-load, adapter-resolution, and build failures.
pub async fn build(
    session: &Session, slice: &str,
) -> Result<slice::orchestrate::BuildOutcome, error::Error> {
    let provider = session.provider();
    let paths = provider.paths();
    let layout = project::config::Layout::new(paths.project_root());
    let config = project::config::ProjectConfig::load(layout.project_dir())?;
    let adapter = project::target_policy::project_adapter(provider, &config, paths)?;
    slice::orchestrate::build(provider, layout, jiff::Timestamp::now(), slice, &adapter.manifest)
        .await
}

/// Drive the merge phase for one slice the way the execute loop does
/// (no composition-replace authorization).
///
/// # Errors
///
/// Propagates the merge orchestration's failures.
pub async fn merge(
    session: &Session, slice: &str,
) -> Result<slice::orchestrate::MergeOutcome, error::Error> {
    let provider = session.provider();
    let layout = project::config::Layout::new(provider.paths().project_root());
    slice::orchestrate::merge(provider, layout, jiff::Timestamp::now(), slice, false).await
}

/// The single `main` binding onto the minimal mock source.
///
/// # Panics
///
/// Panics when the binding JSON stops parsing as a [`SourceAssign`].
#[must_use]
pub fn greeting_binding() -> Vec<SourceAssign> {
    greeting_binding_for("mock")
}

/// The single `main` binding onto the named mock source adapter
/// (for the typed-failure profiles).
///
/// # Panics
///
/// Panics when the binding JSON stops parsing as a [`SourceAssign`].
#[must_use]
pub fn greeting_binding_for(adapter: &str) -> Vec<SourceAssign> {
    let main: SourceAssign = serde_json::from_value(
        json!({ "key": "main", "adapter": adapter, "value": "The greeting service." }),
    )
    .expect("mock binding parses");
    vec![main]
}

/// The adversarial two-source pair: a docs source and a code source,
/// both served by the mock core under different adapter names.
///
/// # Panics
///
/// Panics when a binding JSON stops parsing as a [`SourceAssign`].
#[must_use]
pub fn adversarial_bindings() -> Vec<SourceAssign> {
    ["docs", "code"]
        .map(|key| {
            serde_json::from_value(json!({
                "key": key,
                "adapter": format!("mock-{key}"),
                "value": format!("The {key} source."),
            }))
            .expect("mock binding parses")
        })
        .to_vec()
}

/// A minimal in-memory plan named `test` wrapping `changes`.
#[must_use]
pub fn plan_with_changes(changes: Vec<project::plan::Entry>) -> project::plan::Plan {
    project::plan::Plan {
        name: "test".into(),
        sources: std::collections::BTreeMap::new(),
        entries: changes,
    }
}

/// A minimal plan entry bound to project `default`.
#[must_use]
pub fn change(name: &str) -> project::plan::Entry {
    project::plan::Entry {
        name: name.into(),
        project: Some("default".into()),
        depends_on: vec![],
        sources: vec![],
        context: vec![],
        description: None,
        divergence: None,
        disagreements: Vec::new(),
        authority_override: project::plan::AuthorityOverride::default(),
        allow_composition_replace: false,
    }
}

/// [`change()`] plus a `depends-on` list.
#[must_use]
pub fn change_with_deps(name: &str, deps: &[&str]) -> project::plan::Entry {
    let mut entry = change(name);
    entry.depends_on = deps.iter().map(|s| (*s).into()).collect();
    entry
}

/// Rule ids carried by a failing validate operation's report.
///
/// # Panics
///
/// Panics when `err` is not a report-carrying failure.
#[must_use]
pub fn report_rule_ids(err: &project::handler::Error) -> Vec<String> {
    let project::handler::Error::Report {
        body: project::handler::FailureBody::Findings(body),
        ..
    } = err
    else {
        panic!("expected findings report error, got {err:?}");
    };
    body.report().findings.iter().filter_map(|finding| finding.rule_id.clone()).collect()
}
