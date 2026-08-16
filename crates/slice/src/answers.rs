//! The generated synthesis judgment answer schema.
//!
//! Generated via `schemars` from `SynthesisResponse`, so the wire type
//! the deterministic tail parses stays the single source of truth.

use serde_json::Value;

use crate::SynthesisResponse;

/// The synthesis answer schema.
#[must_use]
pub fn synthesis() -> Value {
    project::answers::root_schema::<SynthesisResponse>(
        "synthesis.schema.json",
        "Emery synthesis answer",
        "Generated judgment-answer schema — generated from the Rust wire types by \
         slice::answers; do not edit. Validates the schema-gated answer to the slice \
         synthesis judgment: the typed `proceed | boundary-escalation` envelope. \
         `proceed` promotes the change-artifact bundle the agent staged into the lent \
         workspace (RFC-96 D10); `boundary-escalation` names affected terminal pairs \
         and a typed rationale. The deterministic tail validates the staged tree and \
         runs the projection kernel only on `proceed`.",
    )
}
