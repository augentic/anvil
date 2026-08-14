//! Schema-gated partition and boundary-review answers.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::tree::Scope;
use crate::profile::Assessment;

/// Wire version stamped on partition and boundary-review answers.
pub const PARTITION_VERSION: u32 = 1;

/// Typed `split | leaf` partition of one open domain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct PartitionResponse {
    /// Schema version ([`PARTITION_VERSION`]).
    pub version: u32,
    /// Partition kind.
    pub kind: PartitionKind,
    /// Child domains of a split. Empty on a leaf.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<Child>,
    /// Singular target on a leaf (or a single-target child).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    /// Terminal slice name on a leaf.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slice: Option<String>,
    /// Ownership envelope. Required on a leaf.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ownership: Vec<String>,
    /// Reviewable acceptance boundary. Required on a leaf.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acceptance: Option<String>,
    /// Contributing `(source, lead)` scopes. Empty inherits the domain.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<Scope>,
    /// Domain or leaf predecessors.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<String>,
    /// Why this partition (or leaf close) is the right cut.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
    /// Closed five-dimension complexity assessment.
    pub assessment: Assessment,
}

/// Local gate kind on a partition answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum PartitionKind {
    /// Internal partition; [`PartitionResponse::children`] is the cut.
    Split,
    /// Terminal domain.
    Leaf,
}

/// One child of a split.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Child {
    /// Stable node id (kebab-case).
    pub id: String,
    /// Contributing `(source, lead)` scopes.
    pub sources: Vec<Scope>,
    /// Singular target binding.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    /// Multi-target binding on an internal child.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub targets: Vec<String>,
    /// Ownership envelope when this child is already a leaf.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ownership: Vec<String>,
    /// Predecessors among siblings or earlier domains.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<String>,
    /// Why this child is a separate domain.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
}

/// One bounded boundary review after a provisional score exceeds
/// the slice-split threshold.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct BoundaryReview {
    /// Schema version ([`PARTITION_VERSION`]).
    pub version: u32,
    /// Review verdict.
    pub verdict: ReviewVerdict,
    /// Close-with-rationale prose, or why the leaf is unready.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
    /// Parents to focused-survey when [`ReviewVerdict::Focus`].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub focus: Vec<FocusParent>,
}

/// Boundary-review outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ReviewVerdict {
    /// Close the leaf and record the rationale.
    Close,
    /// Run focused survey for [`BoundaryReview::focus`] and requeue.
    Focus,
    /// Over-envelope leaf that cannot split; blocks authoring.
    Unready,
}

/// One catalog parent named for focused survey.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct FocusParent {
    /// Plan source key.
    pub source: String,
    /// Catalog lead id within that source.
    pub lead: String,
}
