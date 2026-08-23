//! Closed Evidence authority and claim-kind sets.

use serde::{Deserialize, Serialize};

/// Evidence document authority.
///
/// Default precedence is `intent > documentation > behaviour`.
/// Overrides may vary by claim kind but cannot widen this set.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    strum::Display,
    schemars::JsonSchema,
)]
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

/// Evidence claim kind and authority-override key.
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
    schemars::JsonSchema,
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
    /// `kind: example` — runtime capture.
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
