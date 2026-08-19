//! Shared inert provider for the wire-contract suites: satisfies the
//! router's capability bounds so the grammar assembles and
//! pre-dispatch refusals run, but no test dispatches an adapter seam
//! or the model — those legs are covered over the component seam
//! (`tests/journey.rs`, ADR-0002).

use std::path::PathBuf;

use emery_engine::handler::{Anchor, CachePlacement, ExecutionPaths, Locations};
use emery_engine::resolve::{AdapterSelector, ResolvedSource, Resolver};
use omnia_guest::api::invoke::Invoker;

/// The inert provider: a real anchored layout, unreachable capabilities.
#[derive(Clone, Debug)]
pub struct Inert {
    paths: ExecutionPaths,
}

impl Anchor for Inert {
    fn paths(&self) -> &ExecutionPaths {
        &self.paths
    }
}

impl Resolver for Inert {
    fn resolve_source(
        &self, _selector: &AdapterSelector, _paths: &ExecutionPaths,
    ) -> Result<ResolvedSource, emery_error::Error> {
        unreachable!("the wire suites never dispatch adapter resolution")
    }
}

impl emery_engine::extract::Extract for Inert {
    async fn extract(
        &self, _id: &str, _input: &emery_adapter::seam::SourceInput,
    ) -> Result<emery_adapter::seam::Evidence, emery_error::Error> {
        unreachable!("the wire suites never dispatch the adapter seam")
    }
}

impl omnia_guest::Model for Inert {
    async fn create(
        &self, _request: omnia_guest::model::Request,
    ) -> Result<omnia_guest::model::Reply, omnia_guest::model::Error> {
        unreachable!("the wire suites never dispatch the model")
    }
}

/// The command router over an inert provider anchored at `root`.
pub fn router(
    root: impl Into<PathBuf>,
) -> omnia_guest::api::command::Router<Inert, emery_transport::command::Globals> {
    let root = root.into();
    let locations =
        Locations::explicit(root.join("store"), CachePlacement::Parent(root.join("project-cache")));
    let provider = Inert {
        paths: ExecutionPaths::new(root, locations),
    };
    emery_transport::command::router(Invoker::new("emery", provider)).expect("router")
}
