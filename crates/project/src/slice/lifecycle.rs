//! Projected slice lifecycle labels.
//!
//! Never stored on `metadata.yaml` — CLI surfaces project the rung
//! from artifacts and phase timestamps.

use std::path::Path;

use super::metadata::SliceMetadata;
use crate::build_record::BuildRecord;

/// Lifecycle labels a slice may project.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    strum::Display,
    strum::EnumString,
    strum::VariantNames,
    strum::VariantArray,
)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum LifecycleStatus {
    /// Slice directory created; refine in flight or not yet stamped.
    Refining,
    /// Canonical artifacts validated; ready for build.
    Refined,
    /// Build completed; ready for merge.
    Built,
    /// Specs merged into baseline and slice archived.
    Merged,
    /// Slice discarded without merging.
    Dropped,
}

impl LifecycleStatus {
    /// Project the lifecycle label from phase timestamps and slice
    /// artifacts. Never reads a stored status field.
    #[must_use]
    pub fn project(slice_dir: &Path, metadata: &SliceMetadata) -> Self {
        if metadata.dropped_at.is_some() {
            return Self::Dropped;
        }
        if metadata.merged_at.is_some() {
            return Self::Merged;
        }
        // RFC-86 D27: “built” projects from fact-substrate build
        // records (or the completed_at stamp refine/build write), never
        // from a leftover `build/patch.yaml` path check.
        if metadata.completed_at.is_some() || BuildRecord::present(slice_dir) {
            return Self::Built;
        }
        if metadata.defined_at.is_some()
            || slice_dir.join("model.yaml").is_file()
            || slice_dir.join("spec.md").is_file()
        {
            return Self::Refined;
        }
        Self::Refining
    }
}
