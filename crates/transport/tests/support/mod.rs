//! Shared inert provider for the wire-contract suites: satisfies the
//! router's capability bounds so the grammar assembles and pre-dispatch
//! refusals run; no test dispatches the model (the journey covers that).

use std::future::Future;

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

fn never_dispatched() -> Result<omnia_guest::model::Reply, omnia_guest::model::Error> {
    unreachable!("the wire suites never dispatch the model")
}

/// The command router over the inert provider.
pub fn router() -> omnia_guest::api::command::Router<Inert, emery_transport::command::Globals> {
    emery_transport::command::router(Invoker::new("emery", Inert)).expect("router")
}
