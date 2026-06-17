//! Authority / status / winner derivation kernel.
//!
//! Given a requirement's contributing claims, the per-source document
//! authority classes, the per-slice `authority-override` map, and the
//! agent's `agreement` verdict, [`resolve`] derives the requirement's
//! [`RequirementStatus`], the per-claim winner markers, and the
//! [`ProvenanceResolution`] label the provenance projection recomputes
//! on demand. The function is pure and total: no filesystem, no clock,
//! no panic.
//!
//! Authority is keyed by `(source, kind)`, so one requirement can mix
//! claim kinds. The resolution order walked per claim is:
//!
//! 1. per-slice `authority-override[kind]` → the named source wins
//!    outright for that kind;
//! 2. document-level `authority`;
//! 3. the default class ordering `intent > documentation > behaviour`;
//! 4. a tie at the strictly-greatest effective class → `conflict`.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use specify_model::evidence::{AuthorityClass, ClaimKind};
use specify_model::spec::provenance::RequirementStatus;

use crate::slice::provenance::ProvenanceResolution;

/// The agent's agreement verdict for one requirement.
///
/// `Option<Agreement>` at the call boundary: the verdict is omitted for
/// a requirement with zero or one contributing claim, where there is no
/// agreement to record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, strum::Display)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum Agreement {
    /// Contributing claims describe the same value.
    Agreed,
    /// Contributing claims disagree; the kernel selects a winner by
    /// resolved authority.
    Disagreed,
}

/// One contributing claim, identified by the stable `(source, id,
/// kind)` triple the claim contract pins.
///
/// Resolution consults `source` and `kind`; `id` carries the claim's
/// identity so callers can pass real claim references and correlate the
/// returned winner markers back to `model.yaml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimRef {
    /// Source key (matches a top-level `plan.yaml.sources.<key>`).
    pub source: String,
    /// Claim id within that source's Evidence document.
    pub id: String,
    /// Claim kind (mirrors `schemas/evidence.schema.json#/$defs/claimKind`).
    pub kind: ClaimKind,
}

/// The kernel's verdict for one requirement.
///
/// `winners` is aligned index-for-index with the `claims` slice passed
/// to [`resolve`]: `None` on every claim of an `agreed` / single /
/// unknown requirement (no winner/loser distinction), and `Some(true)`
/// on the winning claim(s) with `Some(false)` on the losers of a
/// `divergence`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolution {
    /// Derived requirement status stamped onto `model.yaml`.
    pub status: RequirementStatus,
    /// Recomputed provenance label (not persisted; the provenance
    /// projection derives it from the same inputs).
    pub resolution: ProvenanceResolution,
    /// Per-claim winner markers, aligned with the input claim order.
    pub winners: Vec<Option<bool>>,
}

impl Resolution {
    /// A resolution with no winner/loser distinction — every claim
    /// carries a `None` marker.
    fn uniform(
        claim_count: usize, status: RequirementStatus, resolution: ProvenanceResolution,
    ) -> Self {
        Self {
            status,
            resolution,
            winners: vec![None; claim_count],
        }
    }
}

/// Derive a requirement's status, winner markers, and provenance label.
///
/// `authority` maps each source key to its document-level
/// [`AuthorityClass`]; `overrides` is the per-slice
/// `authority-override` map (claim kind → winning source key). A source
/// missing from `authority` is treated as the lowest class
/// ([`AuthorityClass::Behaviour`]) — Evidence always carries an
/// `authority`, so this only guards malformed input.
#[must_use]
pub fn resolve(
    claims: &[ClaimRef], authority: &BTreeMap<String, AuthorityClass>,
    overrides: &BTreeMap<ClaimKind, String>, agreement: Option<Agreement>,
) -> Resolution {
    match claims.len() {
        0 => Resolution::uniform(
            0,
            RequirementStatus::Unknown,
            ProvenanceResolution::UnknownNoEvidence,
        ),
        1 => Resolution::uniform(1, RequirementStatus::Agreed, ProvenanceResolution::SingleSource),
        n => match agreement {
            Some(Agreement::Disagreed) => resolve_disagreement(claims, authority, overrides),
            // A well-formed verdict is present for ≥2 claims; an
            // absent one is read as agreement (no winner selection).
            Some(Agreement::Agreed) | None => Resolution::uniform(
                n,
                RequirementStatus::Agreed,
                ProvenanceResolution::SingleValueAgreement,
            ),
        },
    }
}

/// Select the winner among `disagreed` claims by strictly-greatest
/// effective authority. A unique top source yields `divergence`; a tie
/// at the top level yields `conflict`.
fn resolve_disagreement(
    claims: &[ClaimRef], authority: &BTreeMap<String, AuthorityClass>,
    overrides: &BTreeMap<ClaimKind, String>,
) -> Resolution {
    let levels: Vec<Level> =
        claims.iter().map(|claim| effective_level(claim, authority, overrides)).collect();
    // `claims` is non-empty (caller guarantees ≥2), so a max exists.
    let top = levels.iter().copied().max().unwrap_or(Level::Class(0));
    let winning_sources: BTreeSet<&str> = claims
        .iter()
        .zip(&levels)
        .filter(|(_, level)| **level == top)
        .map(|(claim, _)| claim.source.as_str())
        .collect();

    if winning_sources.len() == 1 {
        let winners = levels.iter().map(|level| Some(*level == top)).collect();
        let resolution = if top == Level::Override {
            ProvenanceResolution::PerSliceOverride
        } else {
            ProvenanceResolution::AuthorityResolved
        };
        Resolution {
            status: RequirementStatus::Divergence,
            resolution,
            winners,
        }
    } else {
        Resolution::uniform(
            claims.len(),
            RequirementStatus::Conflict,
            ProvenanceResolution::TiedConflict,
        )
    }
}

/// Effective authority level of one claim, after the per-slice override
/// step. `Override` outranks every document class so a forced source
/// wins outright.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Level {
    /// Document-level authority class, carried as its rank so the
    /// ordering is fixed independent of the `AuthorityClass` enum's
    /// declaration order (`behaviour < documentation < intent`).
    Class(u8),
    /// The per-slice `authority-override` forced this source to win for
    /// its claim's kind.
    Override,
}

/// Resolve one claim's effective level: per-slice override first, then
/// document-level authority class.
fn effective_level(
    claim: &ClaimRef, authority: &BTreeMap<String, AuthorityClass>,
    overrides: &BTreeMap<ClaimKind, String>,
) -> Level {
    if overrides.get(&claim.kind).map(String::as_str) == Some(claim.source.as_str()) {
        return Level::Override;
    }
    let class = authority.get(&claim.source).copied().unwrap_or(AuthorityClass::Behaviour);
    Level::Class(class_rank(class))
}

/// Default class ordering `intent > documentation > behaviour`.
const fn class_rank(class: AuthorityClass) -> u8 {
    match class {
        AuthorityClass::Behaviour => 0,
        AuthorityClass::Documentation => 1,
        AuthorityClass::Intent => 2,
    }
}

#[cfg(test)]
mod tests;
