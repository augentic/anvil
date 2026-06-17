//! Consolidated integration binary for `specify-workflow`.
//!
//! One binary per crate: each former `tests/<area>.rs` hub is pulled in here as
//! a `#[path]` submodule so the crate-under-test links exactly once instead of
//! once per area. The shared `common` helper is declared a single time and every
//! area reaches it as `crate::common`. See
//! [docs/standards/testing.md](../../../docs/standards/testing.md).

mod common;

#[path = "adapter.rs"]
mod adapter;
#[path = "goldens.rs"]
mod goldens;
#[path = "merge_slice.rs"]
mod merge_slice;
#[path = "plan_schema.rs"]
mod plan_schema;
#[path = "registry.rs"]
mod registry;
#[path = "runner.rs"]
mod runner;
#[path = "workspace.rs"]
mod workspace;
