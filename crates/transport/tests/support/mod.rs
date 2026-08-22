//! Shared inert provider for the wire-contract suites: satisfies the
//! router's capability bounds so the grammar assembles and pre-dispatch
//! refusals run; no test dispatches the model (`tests/source.rs` covers
//! that). Storage is the shared scripted in-memory store — empty, so
//! project-scoped verbs refuse `not-initialized`, and inspectable, so
//! suites can assert a refused run wrote nothing.

#[path = "../../../engine/tests/support/storage.rs"]
pub mod storage;

use std::future::Future;
use std::sync::Arc;

use emery_adapter::seam::{Evidence, SourceInput, SourceMetadata};
use emery_adapter::{DispatchError, Source};
use omnia_guest::api::invoke::Invoker;

/// The inert provider: unreachable model and seam capabilities over an
/// inspectable, initially empty scripted store.
#[derive(Clone, Debug, Default)]
pub struct Inert {
    /// The scripted storage backing the provider.
    pub storage: Arc<storage::Memory>,
}

crate::scripted_storage!(Inert, storage);

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

/// The command router over a fresh inert provider.
pub fn router() -> omnia_guest::api::command::Router<Inert, emery_transport::command::Globals> {
    router_over(Inert::default())
}

/// The command router over `provider`, kept by the caller for
/// post-run storage inspection.
pub fn router_over(
    provider: Inert,
) -> omnia_guest::api::command::Router<Inert, emery_transport::command::Globals> {
    emery_transport::command::router(Invoker::new("emery", provider)).expect("router")
}
