//! Evidence authority enums — the closed `AuthorityClass` and
//! `ClaimKind` sets shared by `schemas/evidence.schema.json`,
//! `plan.yaml`'s per-slice `authority-override` map, and the slice
//! model.
//!
//! Authority resolves at document level via the per-Evidence
//! `authority:` field, with one operator override surface — the
//! per-slice `authority-override` on `plan.yaml`, keyed by claim kind
//! (DECISIONS.md §"Authority: document-level plus one override (v1)").
//! A per-Evidence `authority-overrides` map is deferred to a future
//! RFC.

use serde::{Deserialize, Serialize};

/// Closed authority-class enum mirrored from
/// `schemas/evidence.schema.json#/$defs/authorityClass`.
///
/// workflow §Authority hierarchy fixes the default ordering as
/// `intent > documentation > behaviour`; Evidence authority override lifts authority
/// from per-Evidence to per-(Evidence, claim-kind) without widening
/// the class set. New classes still require a workflow contract update.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, strum::Display)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum AuthorityClass {
    /// Operator intent — highest precedence.
    Intent,
    /// Written documentation, design notes, or briefs.
    Documentation,
    /// Observed runtime behaviour (legacy code, runtime captures).
    Behaviour,
}

/// Closed claim-kind enum mirrored from
/// `schemas/evidence.schema.json#/$defs/claimKind`.
///
/// Kept byte-identical with the schema enum so that the per-slice
/// authority override map in `plan.yaml.slices[]` validates against the
/// same closed set. `example` (runtime capture claim) is the runtime
/// capture kind emitted by the `captures` source adapter.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    strum::Display,
    strum::EnumString,
)]
#[serde(rename_all = "kebab-case")]
#[strum(
    serialize_all = "kebab-case",
    parse_err_ty = String,
    parse_err_fn = claim_kind_parse_error
)]
pub enum ClaimKind {
    /// `kind: intent` — operator-stated intent (e.g. `change.md`).
    Intent,
    /// `kind: requirement` — behavioural requirement.
    Requirement,
    /// `kind: criterion` — acceptance criterion.
    Criterion,
    /// `kind: decision` — captured design decision.
    Decision,
    /// `kind: section` — documentation section anchor.
    Section,
    /// `kind: diagram` — diagram or architectural illustration.
    Diagram,
    /// `kind: contract` — interface contract excerpt.
    Contract,
    /// `kind: example` — runtime capture (runtime capture claim, `captures`).
    Example,
    /// `kind: excerpt` — code excerpt.
    Excerpt,
    /// `kind: type` — type definition.
    Type,
    /// `kind: call` — function or method call site.
    Call,
    /// `kind: region` — spatial region (`screenshots`).
    Region,
    /// `kind: container` — spatial container (`screenshots`).
    Container,
    /// `kind: leaf` — spatial leaf (`screenshots`).
    Leaf,
}

fn claim_kind_parse_error(other: &str) -> String {
    format!(
        "`{other}` is not a valid claim kind; expected one of intent, requirement, criterion, \
         decision, section, diagram, contract, example, excerpt, type, call, region, container, leaf"
    )
}

#[cfg(test)]
mod tests;
