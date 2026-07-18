//! Per-leg request telemetry: a transparent [`Model`] wrapper counting
//! how many completion requests each judgment leg issued, and
//! recording each request's model id for effective-model reporting.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use omnia_guest::model::{Error, Format, Model, Reply, Request};

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
    models: Arc<Mutex<Vec<Option<String>>>>,
}

impl<M> Telemetry<M> {
    /// Wrap `inner` with an empty tally.
    pub fn new(inner: M) -> Self {
        Self {
            inner,
            counts: Arc::new(Mutex::new(BTreeMap::new())),
            models: Arc::new(Mutex::new(Vec::new())),
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

    /// The effective model for the run: the model id carried by an
    /// observed request when present, otherwise the composition
    /// root's configured `default` — never a reread of process
    /// environment.
    ///
    /// # Panics
    ///
    /// Panics when the tally lock is poisoned (never in practice).
    #[must_use]
    pub fn effective_model(&self, default: Option<&str>) -> Option<String> {
        self.models
            .lock()
            .expect("the tally is never poisoned")
            .iter()
            .find_map(Clone::clone)
            .or_else(|| default.map(ToString::to_string))
    }
}

impl<M: Model> Model for Telemetry<M> {
    async fn create(&self, request: Request) -> Result<Reply, Error> {
        let leg = match &request.format {
            Format::Schema(schema) => schema.name.clone(),
            Format::Json => "json".to_string(),
            Format::Text => "text".to_string(),
        };
        *self.counts.lock().expect("the tally is never poisoned").entry(leg).or_default() += 1;
        self.models.lock().expect("the tally is never poisoned").push(request.model.clone());
        self.inner.create(request).await
    }
}

/// Report per-leg request counts after a trial run.
///
/// Requests beyond one per leg invocation are repairs — the early
/// signal that a prompt or answer-schema change degraded the model's
/// first answer. The engine legs carry an invocation baseline (one
/// propose per trial, one synthesis per plan entry); adapter legs are
/// reported raw — their invocation counts depend on the authored plan.
pub fn report(counts: &BTreeMap<String, usize>, slices: usize) {
    for (leg, requests) in counts {
        match leg.as_str() {
            "proposal" => {
                let repairs = requests.saturating_sub(1);
                eprintln!("leg proposal: {requests} request(s), {repairs} repair(s)");
            }
            "synthesis" => {
                let repairs = requests.saturating_sub(slices);
                eprintln!(
                    "leg synthesis: {requests} request(s) over {slices} slice(s), \
                     {repairs} repair(s)"
                );
            }
            other => eprintln!("leg {other}: {requests} request(s)"),
        }
    }
}
