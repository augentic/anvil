//! Consolidated integration binary for `schema`.
//!
//! One binary per crate: each former `tests/<area>.rs` is pulled in here as a
//! `#[path]` submodule so the crate-under-test links exactly once. See
//! [docs/standards/testing.md](../../../docs/standards/testing.md).

#[path = "answers.rs"]
mod answers;
#[path = "diagnostics_fingerprint.rs"]
mod diagnostics_fingerprint;
#[path = "diagnostics_report.rs"]
mod diagnostics_report;
#[path = "diagnostics_support.rs"]
mod diagnostics_support;
#[path = "schemas.rs"]
mod schemas;
#[path = "wire_fixtures.rs"]
mod wire_fixtures;
