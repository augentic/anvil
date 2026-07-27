//! Per-leg request telemetry: a transparent [`Model`] wrapper counting
//! how many completion requests each judgment leg issued.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use omnia_guest::model::{Error, Format, Model, Reply, Request};

/// The judgment-leg key for one request: its schema name (`proposal`,
/// `synthesis`), or the format name for a schema-free request.
pub(crate) fn leg(request: &Request) -> String {
    match &request.format {
        Format::Schema(schema) => schema.name.clone(),
        Format::Json => "json".to_string(),
        Format::Text => "text".to_string(),
    }
}

/// A [`Model`] wrapper tallying requests per leg before delegating.
///
/// The leg key is the request's schema name (`proposal`, `synthesis`);
/// a schema-free request counts under its format name. Clones share
/// the tally, so the caller keeps a handle taken before erasure and
/// reads counts back after the run.
#[derive(Clone, Debug)]
pub struct Telemetry<M> {
    inner: M,
    counts: Arc<Mutex<BTreeMap<String, usize>>>,
}

impl<M> Telemetry<M> {
    /// Wrap `inner` with an empty tally.
    pub fn new(inner: M) -> Self {
        Self {
            inner,
            counts: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }

    /// Requests per leg, in leg-name order.
    ///
    /// # Panics
    ///
    /// Panics when the tally lock is poisoned (never in practice).
    #[must_use]
    pub fn counts(&self) -> BTreeMap<String, usize> {
        self.counts.lock().expect("the tally is never poisoned").clone()
    }
}

impl<M: Model> Model for Telemetry<M> {
    async fn create(&self, request: Request) -> Result<Reply, Error> {
        *self
            .counts
            .lock()
            .expect("the tally is never poisoned")
            .entry(leg(&request))
            .or_default() += 1;
        self.inner.create(request).await
    }
}

/// Report per-leg request counts after a case run.
///
/// Requests beyond one per leg invocation are repairs — the early
/// signal that a prompt or answer-schema change degraded the model's
/// first answer. The engine legs carry an invocation baseline (one
/// propose per case, one synthesis per plan entry); adapter legs are
/// reported raw — their invocation counts depend on the authored plan.
pub fn report(counts: &BTreeMap<String, usize>, slices: usize) {
    for (leg, requests) in counts {
        match leg.as_str() {
            "proposal" => {
                let repairs = requests.saturating_sub(1);
                tracing::info!("leg proposal: {requests} request(s), {repairs} repair(s)");
            }
            "synthesis" => {
                let repairs = requests.saturating_sub(slices);
                tracing::info!(
                    "leg synthesis: {requests} request(s) over {slices} slice(s), \
                     {repairs} repair(s)"
                );
            }
            other => tracing::info!("leg {other}: {requests} request(s)"),
        }
    }
}
