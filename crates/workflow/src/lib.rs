//! Specify workflow — slice, change, adapter, registry, config, merge,
//! init lifecycle authority. The artifact model (spec, task, evidence,
//! discovery) and the artifact validation rule registry both live in
//! `artifacts` (`artifacts::validate`). See
//! `docs/standards/architecture.md` for the rationale.

pub mod adapter;
pub mod agents;
pub mod change;
pub mod cmd;
pub mod config;
pub mod decisions;
pub mod deploy;
pub mod design_system;
pub mod hydrate;
pub mod init;
pub mod journal;
pub mod judgment;
pub mod merge;
pub mod name;
pub mod orchestrate;
pub mod platform;
#[cfg(feature = "native")]
pub mod plugins;
pub mod registry;
pub mod schema;
pub mod seam;
pub mod slice;
#[cfg(feature = "native")]
pub mod upgrade;
pub mod verb;

#[cfg(test)]
pub(crate) mod test_cache;

pub use platform::{Platform, parse_platforms_csv};
