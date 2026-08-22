//! Inert provider and storage for wire-contract tests.

use std::future::Future;
use std::sync::Arc;

use emery_adapter::seam::{Evidence, SourceInput, SourceMetadata};
use emery_adapter::{DispatchError, Source};
use emery_testkit::Memory;
use omnia_guest::api::invoke::Invoker;

// Provider capabilities are unreachable in these tests.
#[derive(Clone, Debug, Default)]
pub struct Inert {
    pub storage: Arc<Memory>,
}

emery_testkit::scripted_storage!(Inert, storage);

impl omnia_guest::Model for Inert {
    fn create(
        &self, _request: omnia_guest::model::Request,
    ) -> impl Future<Output = Result<omnia_guest::model::Reply, omnia_guest::model::Error>> {
        std::future::ready(never_dispatched())
    }
}

impl Source for Inert {
    fn extract(
        &self, _id: &str, _input: &SourceInput,
    ) -> impl Future<Output = Result<Evidence, DispatchError>> + Send {
        std::future::ready(never_extracted())
    }

    fn metadata(&self, _id: &str) -> SourceMetadata {
        unreachable!("the wire suites never dispatch the source seam")
    }
}

fn never_dispatched() -> Result<omnia_guest::model::Reply, omnia_guest::model::Error> {
    unreachable!("the wire suites never dispatch the model")
}

fn never_extracted() -> Result<Evidence, DispatchError> {
    unreachable!("the wire suites never dispatch the source seam")
}

pub fn router() -> omnia_guest::api::command::Router<Inert, emery_transport::command::Globals> {
    router_over(Inert::default())
}

// Accept a retained provider for post-run storage inspection.
pub fn router_over(
    provider: Inert,
) -> omnia_guest::api::command::Router<Inert, emery_transport::command::Globals> {
    emery_transport::command::router(Invoker::new("emery", provider)).expect("router")
}
