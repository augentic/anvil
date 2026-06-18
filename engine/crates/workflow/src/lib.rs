//! Specify workflow — slice, change, adapter, registry, config, merge,
//! init lifecycle authority. The artifact model (spec, task, evidence,
//! discovery) and the artifact validation rule registry both live in
//! `specify-model` (`specify_model::validate`). See
//! `docs/standards/architecture.md` for the rationale.

pub mod adapter;
pub mod agents;
pub mod change;
pub mod cmd;
pub mod config;
pub mod decisions;
pub mod design_system;
pub mod init;
pub mod journal;
pub mod merge;
pub mod name;
pub mod plan_lock;
pub mod platform;
pub mod plugins;
pub mod registry;
pub mod schema;
pub mod slice;
pub mod upgrade;

#[cfg(test)]
pub(crate) mod test_cache;

pub use platform::{Platform, parse_platforms_csv};
