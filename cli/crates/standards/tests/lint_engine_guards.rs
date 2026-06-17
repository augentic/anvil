//! Engine/schema invariant guards: the lint engine carries no rule
//! policy, and every accepted hint kind is executable (no reserved
//! kinds, schema and interpreter set stay in lockstep).

#[path = "lint_engine_guards/no_embedded_policy.rs"]
mod no_embedded_policy;
#[path = "lint_engine_guards/no_reserved_hint_kinds.rs"]
mod no_reserved_hint_kinds;
