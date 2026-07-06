//! Consolidated integration binary for `specify-registry`.
//!
//! One binary per crate: each `tests/<area>.rs` hub is pulled in here as a
//! `#[path]` submodule so the crate-under-test links exactly once instead of
//! once per area. See [docs/standards/testing.md](../../../docs/standards/testing.md).

#[path = "pack.rs"]
mod pack;
