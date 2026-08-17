//! Operator corrections: durable `plan.correction.recorded` facts whose
//! activity is projected from the fact union, never stored; the
//! partition judgment receives active ones as hard constraints.

use std::collections::BTreeMap;

use crate::journal::{CorrectionConstraint, Event, EventKind};

/// One operator correction for a decomposition domain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Correction {
    /// Operator intent, verbatim.
    pub intent: String,
    /// Closed structural constraint the deterministic tail enforces.
    pub constraint: Option<CorrectionConstraint>,
    /// Child domain ids a `split` constraint requires.
    pub children: Vec<String>,
}

/// Corrections honored at author re-entry, keyed by domain id and in
/// fact order. Only bound-path facts (no proposal digest) apply — an
/// authored-path correction rides its proposal through `plan amend`.
#[must_use]
pub fn active(events: &[Event]) -> BTreeMap<String, Vec<Correction>> {
    let mut out: BTreeMap<String, Vec<Correction>> = BTreeMap::new();
    for event in events {
        if let EventKind::PlanCorrectionRecorded {
            domain,
            intent,
            constraint,
            children,
            proposal: None,
        } = &event.kind
        {
            out.entry(domain.clone()).or_default().push(Correction {
                intent: intent.clone(),
                constraint: *constraint,
                children: children.clone(),
            });
        }
    }
    out
}
