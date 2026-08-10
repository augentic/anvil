//! Closed source- and target-adapter operation enums.
//!
//! Serde, `strum::Display`, and `strum::EnumString` share the same
//! `kebab-case` rule, so every surface agrees on one wire spelling.

use serde::{Deserialize, Serialize};
use strum::EnumString;

/// Closed source-adapter operation set (`extract | survey`).
///
/// Source adapters serve exactly these two operations per
/// workflow §Source adapter contract — the closed WIT operation set
/// carried by [`crate::adapter::SourceAdapter`] (derived from the
/// axis, not declared on disk; adapters have no manifest).
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

/// Closed target-adapter operation set
/// (`build | guidance | merge | repair | review | verify`).
///
/// Refine-time artifacts are synthesised by core, not produced by an
/// operation, so the set is exactly these six: `guidance` at refine
/// time, the four RFC-90 build-loop phases driven by the engine's
/// phase machine, and `merge` at landing. Variants stay in
/// kebab-alphabetical order so `BTreeMap` iteration matches the wire
/// envelope.
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
    /// Build — generation only, driven by the execute loop's build phase.
    Build,
    /// Guidance — synthesis-time guidance read by core during refine.
    Guidance,
    /// Merge — landing gate, driven by the execute loop's merge phase.
    Merge,
    /// Repair — one findings-directed repair pass (RFC-90).
    Repair,
    /// Review — one engineering-standards review pass (RFC-90).
    Review,
    /// Verify — one check pass over the lent workspace (RFC-90).
    Verify,
}
