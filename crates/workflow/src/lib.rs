//! Specify workflow — slice, change, adapter, registry, config, merge,
//! init lifecycle authority. The artifact model (spec, task, evidence,
//! discovery) and the artifact validation rule registry both live in
//! `artifacts` (`artifacts::validate`). See
//! `docs/standards/architecture.md` for the rationale.

pub mod adapter;
pub(crate) mod agents;
pub mod change;
pub(crate) mod cmd;
pub(crate) mod config;
pub(crate) mod decisions;
pub(crate) mod design_system;
pub(crate) mod fs;
pub mod handler;
pub mod init;
pub mod journal;
pub(crate) mod judgment;
pub(crate) mod merge;
pub(crate) mod name;
pub(crate) mod orchestrate;
pub mod platform;
pub mod registry;
pub(crate) mod schema_gate;
pub mod seam;
pub(crate) mod serde_time;
pub mod slice;
pub mod source;
pub(crate) mod target_policy;

pub use platform::Platform;
