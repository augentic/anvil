//! DTOs mirroring the WIT `source` records.

use std::fmt;

use schemars::JsonSchema;
use serde::Deserialize;

/// Resolve-time source adapter metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceMetadata {
    /// Exact minimum Emery version, if any.
    pub emery_floor: Option<String>,
}

/// Read-only source workspace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceWorkspace {
    /// Opaque preparation identity.
    pub id: String,
    /// Deployment-local view root.
    pub root: String,
}

/// Workspace or inline source content.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceContent {
    /// Read-only view of a location-backed source.
    Workspace(SourceWorkspace),
    /// Inline value without a filesystem lend.
    Value(String),
}

/// Source operation input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceInput {
    /// Binding key.
    pub key: String,
    /// Workspace or inline content.
    pub content: SourceContent,
}

impl SourceInput {
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
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Authority {
    /// Operator directives.
    Intent,
    /// Specifications and documentation.
    Documentation,
    /// Observed behaviour.
    Behaviour,
}

impl fmt::Display for Authority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Intent => "intent",
            Self::Documentation => "documentation",
            Self::Behaviour => "behaviour",
        })
    }
}

/// Closed claim taxonomy; update the workflow contract and schema together.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "kebab-case")]
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

impl fmt::Display for ClaimKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Intent => "intent",
            Self::Requirement => "requirement",
            Self::Criterion => "criterion",
            Self::Decision => "decision",
            Self::Section => "section",
            Self::Diagram => "diagram",
            Self::Contract => "contract",
            Self::Example => "example",
            Self::Excerpt => "excerpt",
            Self::Type => "type",
            Self::Call => "call",
            Self::Region => "region",
            Self::Container => "container",
            Self::Leaf => "leaf",
        })
    }
}

/// Claim backing.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "kebab-case")]
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
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct Claim {
    /// Kind from the closed taxonomy.
    pub kind: ClaimKind,
    /// Stable dotted-kebab ID; required for requirements, criteria, and examples.
    #[serde(default)]
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

// Treat a malformed open field as absent.
fn lenient<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::de::DeserializeOwned,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(T::deserialize(value).ok())
}

/// Extracted claims and their document-level authority.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct Evidence {
    /// Document-level authority.
    pub authority: Authority,
    /// Extracted claims.
    pub claims: Vec<Claim>,
}
