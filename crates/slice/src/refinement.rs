//! Refinement-manifest assembly (RFC-91 D4); the DTO and freshness
//! projection live in [`project::refinement`] (re-exported here) so
//! the status projections can consume them.

mod assemble;

pub use assemble::{TargetInputs, assemble};
pub use project::refinement::{
    BundleEntry, Dependency, Freshness, Inputs, Kind, Live, MISSING_CODE, Manifest, Planning,
    STALE_CODE, VERSION, content_digest, empty_digest, file_digest, findings, freshness,
    freshness_with, latest_archive, live_profile, predecessor_digest,
};
