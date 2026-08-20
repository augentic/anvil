//! Source-axis seam vocabulary mirroring the WIT `source` records:
//! resolve-time metadata, the extract input, and the extract claim-set
//! shape (authority, claim taxonomy, backing).

use serde::Deserialize;

/// A source adapter's metadata — mirrors the WIT `source.metadata`
/// record. Read by the host at resolve time from compiled-in constants.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceMetadata {
    /// Optional host-CLI compatibility floor (exact minimum `emery`
    /// version). Absent means no floor.
    pub emery_floor: Option<String>,
}

/// Read-only source view — mirrors the WIT `source.workspace` record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceWorkspace {
    /// Opaque identity of the preparation.
    pub id: String,
    /// Deployment-local path of the read-only view root.
    pub root: String,
}

/// Workspace-or-value payload — mirrors the WIT `source.content` variant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceContent {
    /// Read-only CID view of a location-backed source.
    Workspace(SourceWorkspace),
    /// Inline value; no filesystem lend.
    Value(String),
}

/// Typed source-operation input — mirrors the WIT `source.input` record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceInput {
    /// Source-binding key.
    pub key: String,
    /// Read-only source view or inline value.
    pub content: SourceContent,
}

impl SourceInput {
    /// Inline-value input — the value extract shape.
    #[must_use]
    pub fn value(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            content: SourceContent::Value(value.into()),
        }
    }
}

/// Document-level authority class for an extracted claim set
/// (`intent` > `documentation` > `behaviour`). Controls who wins a
/// cross-source disagreement.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Authority {
    /// Operator directives. Highest authority.
    Intent,
    /// Written specifications and documentation.
    Documentation,
    /// Empirically observed behaviour. Lowest authority.
    Behaviour,
}

/// Claim-kind taxonomy from `schemas/evidence.schema.json`. New kinds
/// require updating the workflow contract and schemas together.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ClaimKind {
    /// Operator intent statement.
    Intent,
    /// A behavioural requirement.
    Requirement,
    /// An acceptance criterion.
    Criterion,
    /// A recorded decision.
    Decision,
    /// A document section.
    Section,
    /// A diagram.
    Diagram,
    /// An API contract.
    Contract,
    /// Runtime capture claims emitted by the `captures` source adapter.
    Example,
    /// A verbatim excerpt.
    Excerpt,
    /// A type declaration.
    Type,
    /// A call site.
    Call,
    /// A code region.
    Region,
    /// A container (module, package, …).
    Container,
    /// A leaf item.
    Leaf,
}

/// Backing data of a claim — mirrors the WIT `source.backing` variant.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Backing {
    /// A small, verbatim piece of data passed directly.
    Payload(String),
    /// A pointer to a block of data in the filesystem.
    Path(String),
}

/// A claim extracted from a source — mirrors the WIT `source.claim` record.
///
/// The schema leaves per-kind body fields open (`additionalProperties:
/// true`), so unmodeled keys flatten into [`Claim::extras`], and the
/// modeled open fields (`synopsis`, `backing`) deserialize leniently
/// rather than failing the whole answer.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct Claim {
    /// The claim's kind from the closed taxonomy.
    pub kind: ClaimKind,
    /// Stable claim identifier (dotted kebab slug, e.g.
    /// `password-reset.expiry`). Required when `kind` is `requirement`,
    /// `criterion`, or `example`; optional on other kinds.
    #[serde(default)]
    pub id: Option<String>,
    /// Per-claim source anchor: `<path>`, `<path>#L<n>`, or
    /// `<path>#L<start>-L<end>`.
    #[serde(default)]
    pub path: Option<String>,
    /// Headline summarizing the semantic meaning of this evidence.
    #[serde(default, deserialize_with = "lenient")]
    pub synopsis: Option<String>,
    /// Backing data of the claim (a path or a raw payload).
    #[serde(default, deserialize_with = "lenient")]
    pub backing: Option<Backing>,
    /// Open per-kind body fields (`statement`, `criterion`,
    /// `replay-digest`, …), captured verbatim from the answer instead
    /// of being dropped — synthesis reads them from the persisted
    /// Evidence document.
    #[serde(flatten)]
    pub extras: serde_json::Map<String, serde_json::Value>,
}

// Deserialize an open per-kind body field tolerantly: a value that does
// not match the modeled shape is treated as absent.
fn lenient<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::de::DeserializeOwned,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(T::deserialize(value).ok())
}

/// Evidence returned by extract — mirrors the WIT `source.evidence`
/// record: the document-level authority class plus the extracted
/// claim set.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct Evidence {
    /// The document-level authority class of this evidence.
    pub authority: Authority,
    /// The claims extracted from the source.
    pub claims: Vec<Claim>,
}
