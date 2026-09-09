//! Source records
//!
//! The types of the `source` interface itself: what an adapter is given
//! ([`SourceInput`] over a workspace or an inline value), what it reports
//! about itself ([`SourceMetadata`]), and what it returns — an [`Evidence`]
//! document of typed [`Claim`]s with an [`Authority`] class.
//!
//! [`ClaimKind`] is the closed taxonomy the whole system agrees on, and each
//! kind's required extras are declared next to it so the contract states in
//! one place what a complete claim of that kind looks like.

use omnia_guest::schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::claims::DOTTED_KEBAB_PATTERN;

/// Resolve-time source adapter metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceMetadata {
    /// Exact minimum Emery version, if any.
    pub emery_version: Option<String>,
}

/// Workspace or inline source content.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SourceContent {
    /// Deployment-local root of a read-only source view.
    Workspace(String),
    /// Inline value without a filesystem lend.
    Value(String),
}

/// Source operation input.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SourceInput {
    /// Binding key.
    pub key: String,
    /// Workspace or inline content.
    pub content: SourceContent,
}

impl SourceInput {
    /// Creates workspace input over `root`.
    #[must_use]
    pub fn workspace(key: impl Into<String>, root: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            content: SourceContent::Workspace(root.into()),
        }
    }

    /// Creates inline-value input.
    #[must_use]
    pub fn value(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            content: SourceContent::Value(value.into()),
        }
    }
}

/// Claim-set authority, ordered `intent` > `documentation` > `behaviour`.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, JsonSchema, strum::Display)]
#[serde(rename_all = "kebab-case")]
#[schemars(crate = "omnia_guest::schemars")]
#[strum(serialize_all = "kebab-case")]
pub enum Authority {
    /// Operator directives.
    Intent,
    /// Specifications and documentation.
    Documentation,
    /// Observed behaviour.
    Behaviour,
}

impl Authority {
    /// Lower ranks outrank higher ranks (`intent` = 0).
    #[must_use]
    pub const fn rank(self) -> u8 {
        match self {
            Self::Intent => 0,
            Self::Documentation => 1,
            Self::Behaviour => 2,
        }
    }
}

/// Closed claim taxonomy; update the workflow contract and schema together.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, JsonSchema, strum::Display)]
#[serde(rename_all = "kebab-case")]
#[schemars(crate = "omnia_guest::schemars")]
#[strum(serialize_all = "kebab-case")]
pub enum ClaimKind {
    /// Operator intent.
    Intent,
    /// Behavioural requirement.
    Requirement,
    /// Acceptance criterion.
    Criterion,
    /// Recorded decision.
    Decision,
    /// Document section.
    Section,
    /// Diagram.
    Diagram,
    /// API contract.
    Contract,
    /// Runtime capture.
    Example,
    /// Verbatim excerpt.
    Excerpt,
    /// Type declaration.
    Type,
    /// Call site.
    Call,
    /// Code region.
    Region,
    /// Container such as a module or package.
    Container,
    /// Leaf item.
    Leaf,
}

/// Claim backing.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[schemars(crate = "omnia_guest::schemars")]
pub enum Backing {
    /// Inline verbatim data.
    Payload(String),
    /// Filesystem path.
    Path(String),
}

/// A claim extracted from a source.
///
/// Open per-kind fields flatten into [`Claim::extras`]. `synopsis` and
/// `backing` are lenient: malformed shapes become absent.
#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[schemars(crate = "omnia_guest::schemars")]
pub struct Claim {
    /// Kind from the closed taxonomy.
    pub kind: ClaimKind,
    /// Stable dotted-kebab ID; required for requirements, criteria, and examples.
    #[serde(default)]
    #[schemars(regex(pattern = DOTTED_KEBAB_PATTERN))]
    pub id: Option<String>,
    /// Source anchor: `<path>`, `<path>#L<n>`, or `<path>#L<n>-L<n>`.
    #[serde(default)]
    pub path: Option<String>,
    /// Semantic headline.
    #[serde(default, deserialize_with = "lenient")]
    pub synopsis: Option<String>,
    /// Path or inline backing.
    #[serde(default, deserialize_with = "lenient")]
    pub backing: Option<Backing>,
    /// Open per-kind fields preserved for synthesis.
    #[serde(flatten)]
    pub extras: serde_json::Map<String, serde_json::Value>,
}

/// Extracted claims and their document-level authority.
#[derive(Clone, Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[schemars(crate = "omnia_guest::schemars")]
pub struct Evidence {
    /// Document-level authority.
    pub authority: Authority,
    /// Extracted claims.
    pub claims: Vec<Claim>,
}

// Treat a malformed open field as absent.
fn lenient<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::de::DeserializeOwned,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(T::deserialize(value).ok())
}
