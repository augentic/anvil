//! On-disk `<slice_dir>/metadata.yaml` representation.
//!
//! [`SliceMetadata`] is the document, [`Outcome`] is the latest phase return
//! surface read by `/spec:execute`, and [`TouchedSpec`] lists the specs
//! the slice mutates.

use std::path::{Path, PathBuf};

use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use specify_error::Error;

use crate::adapter::TargetOperation;
use crate::slice::OutcomeKind;

/// Basename of the slice working directory under `.specify/`.
pub const SLICES_DIR_NAME: &str = "slices";

/// On-disk representation of `<slice_dir>/metadata.yaml`.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct SliceMetadata {
    /// Target-adapter identifier (e.g. `omnia@1.0.0`).
    pub target: String,
    /// Current lifecycle state.
    pub status: crate::slice::LifecycleStatus,
    /// When the slice was created.
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        with = "specify_error::serde_rfc3339_opt"
    )]
    pub created_at: Option<Timestamp>,
    /// When the slice entered `Refined`.
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        with = "specify_error::serde_rfc3339_opt"
    )]
    pub defined_at: Option<Timestamp>,
    /// When the slice reached `Built`.
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        with = "specify_error::serde_rfc3339_opt"
    )]
    pub completed_at: Option<Timestamp>,
    /// When the slice was merged.
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        with = "specify_error::serde_rfc3339_opt"
    )]
    pub merged_at: Option<Timestamp>,
    /// When the slice was dropped.
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        with = "specify_error::serde_rfc3339_opt"
    )]
    pub dropped_at: Option<Timestamp>,
    /// Human-readable reason for dropping the slice.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub drop_reason: Option<String>,
    /// Specs affected by this slice.
    #[serde(default)]
    pub touched_specs: Vec<TouchedSpec>,
    /// Latest phase outcome. Written atomically by
    /// `crate::merge::slice::commit` (stamps `Success` before the archive move).
    /// History lives in `.specify/journal.jsonl` (workflow §Observability).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<Outcome>,
}

/// Result of a target-adapter operation (shape | build | merge) as
/// recorded in `metadata.yaml`. Read by `/spec:execute` on phase
/// return to decide the next plan transition.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct Outcome {
    /// Which target-adapter operation produced this outcome.
    pub phase: TargetOperation,
    /// Success, failure, or deferred classification. The wire field
    /// name stays `outcome` for back-compat with existing
    /// `metadata.yaml` files and skill JSON consumers; the Rust name
    /// is `kind` so the `Outcome.outcome` field clash with the enclosing
    /// type is gone.
    #[serde(rename = "outcome")]
    pub kind: OutcomeKind,
    /// When the outcome was recorded.
    #[serde(with = "specify_error::serde_rfc3339")]
    pub at: Timestamp,
    /// Short human-readable summary.
    pub summary: String,
    /// Optional additional context (e.g. stderr output).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
}

/// One entry in [`SliceMetadata::touched_specs`].
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct TouchedSpec {
    /// Adapter name (kebab-case).
    pub name: String,
    /// Whether this spec is new or modifies an existing baseline.
    #[serde(rename = "type")]
    pub kind: SpecKind,
}

/// Whether a touched spec is new or a modification of an existing
/// baseline.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, strum::Display,
)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum SpecKind {
    /// A brand-new spec not yet in the baseline.
    New,
    /// A modification of an existing baseline spec.
    Modified,
}

impl SliceMetadata {
    /// Convenience helper: `<slice_dir>/metadata.yaml`.
    #[must_use]
    pub fn path(slice_dir: &Path) -> PathBuf {
        slice_dir.join("metadata.yaml")
    }

    /// Load `metadata.yaml` from a slice directory.
    ///
    /// # Errors
    ///
    /// Returns [`Error::ArtifactNotFound`] (`kind = "metadata.yaml"`)
    /// when the file is absent — the canonical "not a slice directory"
    /// signal that `specify slice list` and `/spec:execute` rely on.
    /// [`Error::YamlDe`] surfaces serde-saphyr deserialisation failures
    /// (malformed YAML, unknown enum tags, type mismatches);
    /// [`Error::Io`] propagates filesystem read errors past the
    /// existence probe (permissions, mid-flight truncation).
    pub fn load(slice_dir: &Path) -> Result<Self, Error> {
        let path = Self::path(slice_dir);
        if !path.exists() {
            return Err(Error::ArtifactNotFound {
                kind: "metadata.yaml",
                path,
            });
        }
        let content = std::fs::read_to_string(&path)?;
        let meta: Self = serde_saphyr::from_str(&content)?;
        Ok(meta)
    }

    /// Atomically write `metadata.yaml` to a slice directory,
    /// overwriting if present. Always trailing-newlined.
    ///
    /// # Errors
    ///
    /// Returns [`Error::YamlSer`] when serde-saphyr fails to encode
    /// `self` — typically a serializer bug rather than a data issue,
    /// since every field of [`SliceMetadata`] is YAML-safe by
    /// construction. Returns [`Error::Io`] when the temp-file create /
    /// write / `sync_all` / atomic rename in
    /// [`specify_model::atomic::yaml_write`] fails. The atomicity
    /// envelope is preserved: a failure here leaves any pre-existing
    /// `metadata.yaml` intact.
    pub fn save(&self, slice_dir: &Path) -> Result<(), Error> {
        let path = Self::path(slice_dir);
        specify_model::atomic::yaml_write(&path, self)
    }
}

#[cfg(test)]
mod tests;
