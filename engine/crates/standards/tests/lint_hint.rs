//! Integration tests for the hint-evaluation umbrella.
//!
//! Per-kind kernel behavior is unit-tested inside each
//! `src/lint/eval/<kind>.rs` module against in-memory models; this
//! crate-level suite keeps only the indexer→umbrella seams: the
//! `path-pattern` candidate-set smoke and the ignore-directive pass.
//! Shared fixtures and runners live in [`eval_support`].

mod eval_support;

#[path = "lint_hint/ignore_directive_pass.rs"]
mod ignore_directive_pass;
#[path = "lint_hint/path_pattern.rs"]
mod path_pattern;
