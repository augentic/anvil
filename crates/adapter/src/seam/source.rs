//! Source-axis seam vocabulary mirroring the WIT `source` records:
//! resolve-time metadata, survey leads, and the extract Evidence shape
//! (authority, claim taxonomy, backing).

use serde::Deserialize;

/// A source adapter's metadata — mirrors the WIT `source.metadata`
/// record. Read by the host at resolve time from compiled-in constants.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceMetadata {
    /// Optional host-CLI compatibility floor (exact minimum `emery`
    /// version). Absent means no floor.
    pub emery_floor: Option<String>,
}

/// The prepared input a source operation reads — the WIT `source-input`.
///
/// The wire carries the tree, never the origin locator: adapters read
/// `Workspace` roots through their own preopens and interpolate
/// `Inline` content into the prompt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceInput {
    /// Deployment-local root of a prepared read-only tree.
    Workspace(String),
    /// Raw content of a single-value binding.
    Inline(String),
}

impl SourceInput {
    /// The prepared tree root, when tree-form.
    #[must_use]
    pub const fn root(&self) -> Option<&str> {
        match self {
            Self::Workspace(root) => Some(root.as_str()),
            Self::Inline(_) => None,
        }
    }

    /// The inlined content, when value-form.
    #[must_use]
    pub const fn content(&self) -> Option<&str> {
        match self {
            Self::Inline(content) => Some(content.as_str()),
            Self::Workspace(_) => None,
        }
    }
}

/// One lead surfaced by a survey — mirrors the WIT `source.lead` record.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct Lead {
    /// Stable kebab-case lead identifier, unique only within its source;
    /// identity is the `(source, lead)` pair. Named `lead` to match the
    /// schema key.
    pub lead: String,
    /// Per-source headline of the lead as this source surfaced it.
    pub synopsis: String,
    /// Agent-authored topic slugs (kebab-case); empty means unclassified.
    #[serde(default)]
    pub topics: Vec<String>,
}

impl Lead {
    /// Render as the survey prompts' lead-block shape for an extract prompt.
    #[must_use]
    pub fn render(&self) -> String {
        let topics = if self.topics.is_empty() {
            String::new()
        } else {
            format!("\n- topics: [{}]", self.topics.join(", "))
        };
        format!("- lead: {}\n- synopsis: {}{topics}", self.lead, self.synopsis)
    }
}

/// Document-level authority class for an Evidence document
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
/// true`), so unmodeled keys are ignored, and the modeled open fields
/// (`synopsis`, `backing`) deserialize leniently rather than failing the
/// whole answer.
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
}

/// Deserialize an open per-kind body field tolerantly: a value that does
/// not match the modeled shape is treated as absent.
fn lenient<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::de::DeserializeOwned,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(T::deserialize(value).ok())
}

/// Evidence returned by extract — mirrors the WIT `source.evidence`
/// record (the canonical Evidence shape minus the envelope `lead` key:
/// the extract call names the lead).
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct Evidence {
    /// The document-level authority class of this evidence.
    pub authority: Authority,
    /// The claims extracted from the source.
    pub claims: Vec<Claim>,
}
