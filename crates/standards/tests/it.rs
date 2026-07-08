//! Consolidated integration binary for `standards`.
//!
//! One binary per crate: each former `tests/<area>.rs` hub is pulled in here as
//! a `#[path]` submodule so the crate-under-test links exactly once. See
//! [docs/standards/testing.md](../../../docs/standards/testing.md).

#[path = "resolve.rs"]
mod resolve;
#[path = "resolve_sort.rs"]
mod resolve_sort;
