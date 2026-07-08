//! Consolidated integration binary for `specify-workflow-lib`.
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
#[path = "author.rs"]
mod author;
#[path = "decisions.rs"]
mod decisions;
#[path = "deploy.rs"]
mod deploy;
#[path = "design_system_parts.rs"]
mod design_system_parts;
#[path = "execute.rs"]
mod execute;
#[path = "goldens.rs"]
mod goldens;
#[path = "hydrate.rs"]
mod hydrate;
#[path = "judgment.rs"]
mod judgment;
#[path = "merge_composition.rs"]
mod merge_composition;
#[path = "merge_slice.rs"]
mod merge_slice;
#[path = "orchestrate.rs"]
mod orchestrate;
#[path = "plan_schema.rs"]
mod plan_schema;
#[path = "propose_topology.rs"]
mod propose_topology;
#[path = "registry.rs"]
mod registry;
#[path = "runner.rs"]
mod runner;
#[path = "synthesis_baseline.rs"]
mod synthesis_baseline;
#[cfg(feature = "native")]
#[path = "upgrade.rs"]
mod upgrade;
#[path = "workspace.rs"]
mod workspace;
