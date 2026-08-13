//! Generated judgment answer schemas, produced via `schemars` from
//! the response wire types so the types the deterministic tails parse
//! stay the single source of truth.

use serde_json::Value;

use crate::judgment::correlate::CorrelationResponse;
use crate::judgment::propose::ProposalResponse;

/// The correlation answer schema.
#[must_use]
pub fn correlation() -> Value {
    project::answers::root_schema::<CorrelationResponse>(
        "correlation.schema.json",
        "Emery correlation answer",
        "Generated judgment-answer schema — generated from the Rust wire types by \
         system::answers; do not edit. Validates the schema-gated answer to the system \
         correlation judgment: the `kind: response` envelope carrying the composed `as-is` \
         element and relationship set. The deterministic tail re-parses the answer, validates \
         the state, and closes cited claims against the persisted Evidence corpus.",
    )
}

/// The initial-plan proposal answer schema.
#[must_use]
pub fn proposal() -> Value {
    project::answers::root_schema::<ProposalResponse>(
        "proposal.schema.json",
        "Emery system-plan proposal answer",
        "Generated judgment-answer schema — generated from the Rust wire types by \
         system::answers; do not edit. Validates the schema-gated answer to the initial \
         system-plan proposal judgment: the `kind: response` envelope carrying the proposed \
         `target` state, optional `transition-*` states, modernization dispositions, and \
         exactly one migration wave. The deterministic tail re-parses the answer, validates \
         every proposed state, and closes claims, evidence scopes, and decision references \
         against the live definition home.",
    )
}
