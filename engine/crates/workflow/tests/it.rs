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
#[path = "agents_fences_render.rs"]
mod agents_fences_render;
#[path = "agents_lock.rs"]
mod agents_lock;
#[path = "decisions.rs"]
mod decisions;
#[path = "design_system_parts.rs"]
mod design_system_parts;
#[path = "goldens.rs"]
mod goldens;
#[path = "materialize_scope.rs"]
mod materialize_scope;
#[path = "merge_composition.rs"]
mod merge_composition;
#[path = "merge_slice.rs"]
mod merge_slice;
#[path = "plan_schema.rs"]
mod plan_schema;
#[path = "propose_topology.rs"]
mod propose_topology;
#[path = "registry.rs"]
mod registry;
#[path = "runner.rs"]
mod runner;
#[path = "upgrade.rs"]
mod upgrade;
#[path = "workspace.rs"]
mod workspace;
