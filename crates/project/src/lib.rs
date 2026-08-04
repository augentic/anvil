//! Emery project foundation — the deployment-neutral layer under the
//! slice and change loops: init, adapter resolution, config, journal,
//! registry, the plan and slice-metadata data models, the seam
//! capability traits, the judgment kernel, and the shared handler
//! plumbing. The artifact model (spec, task, evidence, discovery) and
//! the artifact validation rule registry both live in `artifacts`
//! (`artifacts::validate`). See `docs/standards/architecture.md` for
//! the rationale.

pub mod adapter;
pub(crate) mod agents;
pub mod answers;
pub(crate) mod cmd;
pub mod config;
pub mod decisions;
pub mod fs;
pub mod handler;
pub mod init;
pub mod journal;
pub mod judgment;
pub mod name;
pub mod plan;
pub mod platform;
pub mod registry;
pub mod seam;
pub mod serde_time;
pub mod slice;
pub mod snapshot;
pub mod target_policy;
#[cfg(not(target_arch = "wasm32"))]
pub mod workspace;

pub use platform::Platform;
