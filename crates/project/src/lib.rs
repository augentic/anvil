//! Emery project foundation.
//!
//! The deployment-neutral layer under the slice and change loops; the
//! artifact model and validation rule registry live in `artifacts`.

pub mod adapter;
pub(crate) mod agents;
pub mod answers;
pub mod build_record;
pub mod config;
pub mod decisions;
pub mod fs;
pub mod gap_policy;
pub mod handler;
pub mod identity;
pub mod init;
pub mod journal;
pub mod judgment;
pub mod name;
pub mod plan;
pub mod platform;
pub mod seam;
pub mod serde_time;
pub mod slice;
pub mod snapshot;
pub mod target_policy;
pub mod wave;
pub mod workspace;

pub use gap_policy::GapPolicy;
pub use platform::Platform;
