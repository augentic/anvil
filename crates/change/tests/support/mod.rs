//! Suite-local source bindings and plan builders over `change` wire
//! types.
//!
//! The builders return `change` types, so they live with the owning
//! suites rather than in the mock crate, keeping `mock` free of
//! workflow-crate dependencies.

#![allow(dead_code, reason = "each test binary uses a subset of the shared support surface")]

use change::plan::wire::SourceAssign;
use error::Error;
use project::adapter::{
    AdapterSelector, FIRST_PARTY_NAMESPACE, ResolvedSource, ResolvedTarget, Resolver,
};
use project::handler::{Anchor, ExecutionPaths};
use project::seam::{Evidence, Lead, Source};
use serde_json::json;

/// A provider wrapper widening bare cache-miss selectors to the
/// embedded first-party adapter train — the component deployment's
/// `expand` policy over the session's native provider, which keeps
/// the identity default by design.
///
/// The native catalog accepts only its exact compiled identities, and
/// the mock adapters carry the `0.0.0` development placeholder — so a
/// widened `emery:mock*@<train>` pin could never ensure natively.
/// `ensure_*` therefore narrows a train pin on a `mock*` name back to
/// the bare catalog identity, letting the author flow proceed while
/// any other widened name (e.g. the `--intent` sugar's `intent`)
/// still refuses as unlinked.
#[derive(Clone)]
pub struct Expanding<P>(pub P);

impl<P: Anchor> Anchor for Expanding<P> {
    fn paths(&self) -> &ExecutionPaths {
        self.0.paths()
    }
}

impl<P: omnia_guest::Model> omnia_guest::Model for Expanding<P> {
    async fn create(
        &self, request: omnia_guest::model::Request,
    ) -> Result<omnia_guest::model::Reply, omnia_guest::model::Error> {
        self.0.create(request).await
    }
}

impl<P: Source> Source for Expanding<P> {
    async fn survey(&self, id: String) -> Result<Vec<Lead>, project::seam::Error> {
        self.0.survey(id).await
    }

    async fn extract(&self, id: String, lead: Lead) -> Result<Evidence, project::seam::Error> {
        self.0.extract(id, lead).await
    }
}

impl<P: Resolver> Resolver for Expanding<P> {
    fn expand(&self, selector: &AdapterSelector, paths: &ExecutionPaths) -> AdapterSelector {
        project::adapter::expand_bare(selector, paths)
    }

    fn resolve_source(
        &self, selector: &AdapterSelector, paths: &ExecutionPaths,
    ) -> Result<ResolvedSource, Error> {
        self.0.resolve_source(selector, paths)
    }

    fn resolve_target(
        &self, selector: &AdapterSelector, paths: &ExecutionPaths,
    ) -> Result<ResolvedTarget, Error> {
        self.0.resolve_target(selector, paths)
    }

    async fn ensure_source(
        &self, selector: &AdapterSelector, paths: &ExecutionPaths,
    ) -> Result<ResolvedSource, Error> {
        self.0.ensure_source(&narrow(selector), paths).await
    }

    async fn ensure_target(
        &self, selector: &AdapterSelector, paths: &ExecutionPaths,
    ) -> Result<ResolvedTarget, Error> {
        self.0.ensure_target(&narrow(selector), paths).await
    }
}

/// Narrow a widened first-party train pin on a mock catalog identity
/// back to its bare form for the native ensure.
fn narrow(selector: &AdapterSelector) -> AdapterSelector {
    match selector {
        AdapterSelector::Package { namespace, name, .. }
            if namespace == FIRST_PARTY_NAMESPACE && name.starts_with("mock") =>
        {
            AdapterSelector::Bare { name: name.clone() }
        }
        other => other.clone(),
    }
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
        lifecycle: project::plan::Lifecycle::Pending,
        sources: std::collections::BTreeMap::new(),
        entries: changes,
    }
}

/// A minimal plan entry bound to project `default`.
#[must_use]
pub fn change(name: &str, status: project::plan::Status) -> project::plan::Entry {
    project::plan::Entry {
        name: name.into(),
        project: Some("default".into()),
        status,
        depends_on: vec![],
        sources: vec![],
        context: vec![],
        description: None,
        divergence: None,
        disagreements: Vec::new(),
        authority_override: project::plan::AuthorityOverride::default(),
    }
}

/// [`change()`] plus a `depends-on` list.
#[must_use]
pub fn change_with_deps(
    name: &str, status: project::plan::Status, deps: &[&str],
) -> project::plan::Entry {
    let mut entry = change(name, status);
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
    let project::handler::Error::Report { body, .. } = err else {
        panic!("expected report error, got {err:?}");
    };
    body.report().findings.iter().filter_map(|finding| finding.rule_id.clone()).collect()
}
