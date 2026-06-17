//! Consolidated integration binary for `specify-schema`.
//!
//! One binary per crate: each former `tests/<area>.rs` is pulled in here as a
//! `#[path]` submodule so the crate-under-test links exactly once. See
//! [docs/standards/testing.md](../../../docs/standards/testing.md).

#[path = "schemas.rs"]
mod schemas;
#[path = "wire_fixtures.rs"]
mod wire_fixtures;
