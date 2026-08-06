//! On-disk `<slice_dir>/metadata.yaml` representation.
//!
//! [`SliceMetadata`] is the document, [`Outcome`] is the latest phase
//! return surface read by the `emery plan execute` loop, and
//! [`TouchedSpec`] lists the specs the slice mutates.

use std::path::{Path, PathBuf};

use error::Error;
use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use crate::adapter::TargetOperation;
use crate::slice::OutcomeKind;

/// Basename of the slice working directory under `.emery/`.
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
        with = "crate::serde_time::rfc3339_opt"
    )]
    pub created_at: Option<Timestamp>,
    /// When the slice entered `Refined`.
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        with = "crate::serde_time::rfc3339_opt"
    )]
    pub defined_at: Option<Timestamp>,
    /// When the slice reached `Built`.
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        with = "crate::serde_time::rfc3339_opt"
    )]
    pub completed_at: Option<Timestamp>,
    /// When the slice was merged.
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        with = "crate::serde_time::rfc3339_opt"
    )]
    pub merged_at: Option<Timestamp>,
    /// When the slice was dropped.
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        with = "crate::serde_time::rfc3339_opt"
    )]
    pub dropped_at: Option<Timestamp>,
    /// Human-readable reason for dropping the slice.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub drop_reason: Option<String>,
    /// Specs affected by this slice.
    #[serde(default)]
    pub touched_specs: Vec<TouchedSpec>,
    /// Latest phase outcome. Written atomically by
    /// the merge commit tail (stamps `Success` before the archive move).
    /// History lives in `.emery/events/<actor>.jsonl` (workflow §Observability).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome: Option<Outcome>,
}

/// Result of a target-adapter operation (guidance | build | merge) as
/// recorded in `metadata.yaml`. Read by the execute loop on phase
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
    #[serde(with = "crate::serde_time::rfc3339")]
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

/// The `slice-not-found` failure for an absent slice directory: the
/// detail names the missing slice and lists its siblings so a typo'd
/// name reads as a typo, not a corrupt tree.
#[must_use]
pub fn slice_not_found(slice_dir: &Path) -> Error {
    let name = slice_dir.file_name().map(|n| n.to_string_lossy()).unwrap_or_default();
    let siblings = slice_dir.parent().map(sibling_slices).unwrap_or_default();
    let inventory = if siblings.is_empty() {
        "no slices exist".to_string()
    } else {
        format!("available: {}", siblings.join(", "))
    };
    Error::Diag {
        code: "slice-not-found",
        detail: format!(
            "no slice named `{name}` under {} ({inventory})",
            parent_display(slice_dir)
        ),
    }
}

/// Sorted directory names under `slices_dir` — best-effort: an
/// unreadable root yields the empty inventory rather than masking the
/// not-found failure.
fn sibling_slices(slices_dir: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(slices_dir) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .filter_map(Result::ok)
        .filter(|e| e.path().is_dir())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort_unstable();
    names
}

fn parent_display(slice_dir: &Path) -> String {
    slice_dir.parent().map_or_else(|| slice_dir.display().to_string(), |p| p.display().to_string())
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
    /// Returns the `slice-not-found` diagnostic ([`slice_not_found`])
    /// when the slice directory itself is absent — the operator-typo
    /// signal, listing the sibling slices. Returns
    /// [`Error::ArtifactNotFound`] (`kind = "metadata.yaml"`) when the
    /// directory exists but the file is absent — the "not a slice
    /// directory" signal. [`Error::YamlDe`] surfaces serde-saphyr
    /// deserialisation failures (malformed YAML, unknown enum tags,
    /// type mismatches); [`Error::Io`] propagates filesystem read
    /// errors past the existence probe (permissions, mid-flight
    /// truncation).
    pub fn load(slice_dir: &Path) -> Result<Self, Error> {
        if !slice_dir.is_dir() {
            return Err(slice_not_found(slice_dir));
        }
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
    /// [`artifacts::atomic::yaml_write`] fails. The atomicity
    /// envelope is preserved: a failure here leaves any pre-existing
    /// `metadata.yaml` intact.
    pub fn save(&self, slice_dir: &Path) -> Result<(), Error> {
        let path = Self::path(slice_dir);
        artifacts::atomic::yaml_write(&path, self)
    }
}
