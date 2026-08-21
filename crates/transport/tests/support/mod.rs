//! Shared inert provider for the wire-contract suites: satisfies the
//! router's capability bounds so the grammar assembles and pre-dispatch
//! refusals run; no test dispatches the model (`tests/native.rs` covers that).

use std::future::Future;

use emery_adapter::seam::{Evidence, SourceInput, SourceMetadata};
use emery_adapter::{DispatchError, SourceDispatch};
use omnia_guest::api::invoke::Invoker;

/// The inert provider: unreachable capabilities.
#[derive(Clone, Debug)]
pub struct Inert;

impl omnia_guest::Model for Inert {
    fn create(
        &self, _request: omnia_guest::model::Request,
    ) -> impl Future<Output = Result<omnia_guest::model::Reply, omnia_guest::model::Error>> {
        std::future::ready(never_dispatched())
    }
}

impl SourceDispatch for Inert {
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

/// The command router over the inert provider.
pub fn router() -> omnia_guest::api::command::Router<Inert, emery_transport::command::Globals> {
    emery_transport::command::router(Invoker::new("emery", Inert)).expect("router")
}
