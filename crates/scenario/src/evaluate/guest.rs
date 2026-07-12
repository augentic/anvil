//! Pure evaluators for the `guest-execute-loop` registered probes.
//!
//! The canonical scenario declares two `kind: registered` assertions.
//! The journal cadence is a filesystem read and ships here; generated
//! crate verification spawns `cargo check` and is registered by the
//! owning harness instead.

use std::fs;

use crate::grade::{Execution, Verdict};

/// Journal events the guest loop must emit for one drained slice.
const CADENCE_EVENTS: [&str; 2] = ["slice.merge.succeeded", "slice.archive.created"];

/// The guest loop journalled the merge and archive events over the
/// `"."` preopen (`guest-journal-cadence`).
#[must_use]
pub fn journal_cadence(execution: &Execution) -> Verdict {
    let evidence = ".specify/journal.jsonl";
    let path = execution.root().join(evidence);
    let Ok(journal) = fs::read_to_string(&path) else {
        return Verdict::fail(evidence, format!("journal not readable at {}", path.display()));
    };
    let missing: Vec<&str> = CADENCE_EVENTS
        .iter()
        .filter(|event| !journal.contains(&format!("\"{event}\"")))
        .copied()
        .collect();
    if missing.is_empty() {
        Verdict::pass(evidence)
    } else {
        Verdict::fail(evidence, format!("journal is missing {}", missing.join(", ")))
    }
}
