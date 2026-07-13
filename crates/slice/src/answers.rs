//! The generated synthesis judgment answer schema.
//!
//! Generated (via `schemars`) from `SynthesisResponse` — the same type
//! the deterministic tail parses the raw answer through — so the Rust
//! wire type stays the single source of truth. The committed copy under
//! `crates/slice/answers/synthesis.schema.json` is parity-gated by
//! `crates/slice/tests/answers.rs`.

use serde_json::Value;

use crate::SynthesisResponse;

/// The synthesis answer schema.
#[must_use]
pub fn synthesis() -> Value {
    project::answers::root_schema::<SynthesisResponse>(
        "synthesis.schema.json",
        "Specify synthesis answer",
        "Generated judgment-answer schema — generated from the Rust wire types by \
         slice::answers; do not edit. Validates the schema-gated answer to the slice \
         synthesis judgment: the `kind: response` envelope carrying the structured model and \
         the prose-only Markdown artifacts. The deterministic tail re-parses the answer and \
         runs the projection kernel.",
    )
}
