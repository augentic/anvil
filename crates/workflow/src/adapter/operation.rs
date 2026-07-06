//! Closed source- and target-adapter operation enums.
//!
//! [`SourceOperation`] (`extract | survey`) and [`TargetOperation`]
//! (`shape | build | merge`) are the typed `briefs.keys()` carried by
//! the axis-specific manifest structs in `core.rs`. Living together in
//! this module keeps the source/target operation pair symmetric.
//!
//! Wire format is kebab-case on both sides (`extract | survey` /
//! `shape | build | merge`) — the [`Serialize`] / [`Deserialize`]
//! derives, the [`strum::Display`] impl, and the [`strum::EnumString`]
//! impl all share the same `kebab-case` rule, so the YAML manifest
//! key, the slice outcome `phase` field, and any
//! `parse::<TargetOperation>()` call all agree on a single wire
//! spelling.

use serde::{Deserialize, Serialize};
use strum::EnumString;

/// Closed source-adapter operation set (`extract | survey`).
///
/// Source adapters serve exactly these two operations per
/// workflow §Source adapter contract — the closed WIT operation set
/// carried by [`crate::adapter::SourceAdapter`] (derived from the
/// axis, not declared on disk; RFC-64 adapters have no manifest).
///
/// Variants declared in kebab-alphabetical order so `BTreeMap`
/// iteration matches the wire envelope.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    EnumString,
    strum::Display,
)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum SourceOperation {
    /// Slice-time evidence extraction.
    Extract,
    /// Plan-time lead discovery.
    Survey,
}

impl SourceOperation {
    /// Staged-artifact filename per operation: `evidence.yaml` for
    /// `extract`, `leads.md` for `survey`.
    #[must_use]
    pub const fn artifact_name(self) -> &'static str {
        match self {
            Self::Extract => "evidence.yaml",
            Self::Survey => "leads.md",
        }
    }
}

/// Closed target-adapter operation set (`shape | build | merge`).
///
/// Target adapters serve exactly these three operations per
/// workflow §Target adapter contract — the closed WIT operation set
/// carried by [`crate::adapter::TargetAdapter`] (derived from the
/// axis, not declared on disk; RFC-64 adapters have no manifest) and
/// the discriminant stamped into per-slice outcomes
/// (`<slice_dir>/metadata.yaml.outcome.phase`).
///
/// Refine-time artifacts are synthesised by core, not produced by an
/// operation, so the set is exactly these three target operations.
///
/// Variants declared in kebab-alphabetical order so `BTreeMap`
/// iteration matches the wire envelope.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    Serialize,
    Deserialize,
    EnumString,
    strum::Display,
)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum TargetOperation {
    /// Build — implementation, driven by `/spec:build`.
    Build,
    /// Merge — landing gate, driven by `/spec:merge`.
    Merge,
    /// Shape — synthesis-time guidance read by core during `/spec:refine`.
    Shape,
}
