//! Golden + fixture tests for the workflow contract: the merge-engine
//! goldens. The `validate_slice` `DiagnosticReport` shape is pinned at
//! the binary level by `tests/e2e.rs` (`validate-good.json` /
//! `validate-bad.json`). Wire-schema accept/reject fixtures live in the
//! schema crate (`crates/schema/tests/wire_fixtures.rs`). Shared helpers
//! live in [`common`].

mod common;

#[path = "goldens/merge_engine.rs"]
mod merge_engine;
