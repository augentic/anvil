//! The Specify-owned harness adapter's native core: one deterministic,
//! model-free library implementing both `specify:adapter` axes for
//! engine tests. Its types mirror the WIT `specify:adapter/types`
//! records so both consumers (the native test provider and the
//! WASM adapter component) stay thin mapping layers.
//!
//! Behaviour keys off the routed adapter id — the profile catalog
//! lives in the package `README.md`. Builds and merge gates also
//! honour the per-project [`FAIL_BUILD_MARKER`] /
//! [`FAIL_MERGE_PREFLIGHT_MARKER`] / [`FAIL_MERGE_POSTFLIGHT_MARKER`]
//! files so interruption tests can park and resume without rebinding.

mod source;
mod targets;

pub use source::{Authority, Backing, Claim, ClaimKind, Evidence, Lead, extract, survey};
pub use targets::{
    BUILD_DIR, FAIL_BUILD_MARKER, FAIL_MERGE_POSTFLIGHT_MARKER, FAIL_MERGE_PREFLIGHT_MARKER, Input,
    MergePhase, Output, Platform, PlatformsCapability, Report, Status, build, build_artifact_path,
    guidance, merge, target_platforms,
};

/// Typed adapter failure, mirroring the WIT `types.error` variant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// The request itself is malformed; retrying unchanged is pointless.
    InvalidRequest(String),
    /// A filesystem operation failed on the adapter side.
    Io(String),
    /// An internal adapter step failed.
    Internal(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidRequest(detail) => write!(f, "invalid request: {detail}"),
            Self::Io(detail) => write!(f, "io: {detail}"),
            Self::Internal(detail) => write!(f, "internal: {detail}"),
        }
    }
}

impl std::error::Error for Error {}
