//! Seam vocabulary mirroring the `specify:adapter` WIT records.
//!
//! Only answer-deserialized types carry serde derives.

use std::path::{Path, PathBuf};

use omnia_guest::model::McpGrant;
use serde::Deserialize;

/// Operation error — mirrors the WIT `types.error` variant.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// The request itself is malformed.
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    /// A filesystem operation failed.
    #[error("io: {0}")]
    Io(String),
    /// A judgment call or answer-handling step failed.
    #[error("internal: {0}")]
    Internal(String),
}

impl From<omnia_guest::model::Error> for Error {
    fn from(err: omnia_guest::model::Error) -> Self {
        match err {
            omnia_guest::model::Error::InvalidRequest(detail) => Self::InvalidRequest(detail),
            other => Self::Internal(other.to_string()),
        }
    }
}

/// Call-scoped environment the shim resolves and hands to every operation.
#[derive(Clone, Copy, Debug)]
pub struct Context<'a> {
    /// Adapter identity this call was routed by, e.g. `target:contracts`.
    pub adapter_id: &'a str,
    /// The guest's `"."` preopen root (the shared project mount).
    pub project_root: &'a Path,
    /// The adapter's MCP references endpoint, granted to the spawned
    /// agent so it can fetch `doc://` references lazily.
    pub mcp_url: Option<&'a str>,
}

impl<'a> Context<'a> {
    /// Guest-side context rooted at the guest's own `"."` preopen.
    #[must_use]
    pub fn guest(adapter_id: &'a str, mcp_url: Option<&'a str>) -> Self {
        Self {
            adapter_id,
            project_root: Path::new("."),
            mcp_url,
        }
    }

    /// MCP grants offered on every judgment leg: the adapter's own
    /// references, when an endpoint is set. Named `<name>-references`
    /// after the axis-stripped adapter id.
    #[must_use]
    pub fn grants(&self) -> Vec<McpGrant> {
        let name = self.adapter_id.rsplit(':').next().unwrap_or(self.adapter_id);
        self.mcp_url
            .map(|url| McpGrant {
                name: format!("{name}-references"),
                tools: Vec::new(),
                url: url.to_string(),
            })
            .into_iter()
            .collect()
    }

    /// Resolve an operation's tree root beneath the shared mount.
    #[must_use]
    pub fn tree_root(&self, tree: &WorkingTree) -> PathBuf {
        tree.subpath
            .as_deref()
            .map_or_else(|| self.project_root.to_path_buf(), |sub| self.project_root.join(sub))
    }
}

/// One slice-artifact input — mirrors the WIT `target.input` variant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Input {
    /// The slice's `proposal.md`.
    Proposal(String),
    /// The slice's `design.md`.
    Design(String),
    /// The slice's `tasks.md`.
    Tasks(String),
    /// One behavioural spec (`specs/<domain>/spec.md`).
    Spec(String),
    /// Any additional artifact.
    Other(String),
}

impl Input {
    /// The input's prompt-section label (`proposal`, `design`, …).
    #[must_use]
    pub const fn label(&self) -> &'static str {
        match self {
            Self::Proposal(_) => "proposal",
            Self::Design(_) => "design",
            Self::Tasks(_) => "tasks",
            Self::Spec(_) => "spec",
            Self::Other(_) => "other",
        }
    }

    /// The input's artifact body.
    #[must_use]
    pub fn body(&self) -> &str {
        match self {
            Self::Proposal(body)
            | Self::Design(body)
            | Self::Tasks(body)
            | Self::Spec(body)
            | Self::Other(body) => body,
        }
    }
}

/// The tree an operation works on — mirrors the WIT `working-tree` record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkingTree {
    /// The snapshot the operation applies against.
    pub base: String,
    /// Optional path beneath the shared mount root.
    pub subpath: Option<String>,
}

/// Which side of the deterministic core merge a `merge` dispatch runs
/// on — mirrors the WIT `target.merge-phase` enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MergePhase {
    /// Before the deterministic commit: staged checks over the built
    /// slice. A blocking finding aborts the merge with the slice still
    /// `built`.
    Preflight,
    /// After the deterministic commit: merged-baseline validators over
    /// the updated tree. A blocking finding is a terminal diagnostic,
    /// never a rollback.
    Postflight,
}

impl std::fmt::Display for MergePhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Preflight => "preflight",
            Self::Postflight => "postflight",
        })
    }
}

/// Review severity, ordered for sort stability.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum Severity {
    /// Must fix; blocks success.
    Critical,
    /// Should fix; blocks success.
    Important,
    /// Advisory; never blocks.
    Suggestion,
    /// Take-it-or-leave-it; never blocks.
    Optional,
}

impl Severity {
    /// Whether a finding at this severity blocks a `success` report.
    #[must_use]
    pub const fn blocking(self) -> bool {
        matches!(self, Self::Critical | Self::Important)
    }
}

/// Operation outcome.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    /// The operation completed; findings, if any, are non-blocking.
    Success,
    /// The operation did not complete cleanly.
    Failure,
}

/// Target platform taxonomy.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Platform {
    /// Shared core.
    Core,
    /// iOS shell.
    Ios,
    /// Android shell.
    Android,
    /// Web shell.
    Web,
    /// Desktop shell.
    Desktop,
}

/// One per-platform build output declared by the answer.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct BuildOutput {
    /// Platform this output was produced for.
    pub platform: Platform,
    /// Relative path (from the project root) to the produced artifact.
    pub path: String,
}

/// Per-slice UI-surface signal.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct UiSurface {
    /// Count of screen-bearing requirements the slice introduces or modifies.
    pub screens: u32,
}

/// One diagnostic — mirrors the WIT `finding` record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    /// Rule identifier, absent for findings that cite no codex policy.
    pub rule_id: Option<String>,
    /// Review severity.
    pub severity: Severity,
    /// Folded `title` / `impact` / `remediation` prose.
    pub detail: String,
}

impl Finding {
    /// A blocking (`important`) finding citing no rule — the shape the
    /// deterministic gates emit.
    #[must_use]
    pub fn blocking(detail: impl Into<String>) -> Self {
        Self {
            rule_id: None,
            severity: Severity::Important,
            detail: detail.into(),
        }
    }
}

/// Judgment returned by `build` and `merge` — mirrors the WIT `report`
/// record. The resulting state lives in the working tree, not here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    /// Operation outcome.
    pub status: Status,
    /// Compact findings.
    pub findings: Vec<Finding>,
    /// Per-platform build outputs.
    pub outputs: Vec<BuildOutput>,
    /// Optional UI-surface signal.
    pub ui_surface: Option<UiSurface>,
}

impl Report {
    /// A clean success report — the shape a deterministic merge gate
    /// answers with when it raises no findings and runs no judgment leg.
    #[must_use]
    pub const fn success() -> Self {
        Self {
            status: Status::Success,
            findings: Vec::new(),
            outputs: Vec::new(),
            ui_surface: None,
        }
    }
}

/// A source adapter's metadata — mirrors the WIT `source.metadata`
/// record. Read by the host at resolve time from compiled-in constants.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceMetadata {
    /// Optional host-CLI compatibility floor (exact minimum `specify`
    /// version). Absent means no floor.
    pub specify_floor: Option<String>,
}

/// One adapter-declared build input — mirrors the WIT
/// `target.build-input` record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildInput {
    /// Slice-tree-relative path of the input.
    pub path: String,
    /// Whether the build must abort when the path is absent.
    pub required: bool,
}

/// Declarative platforms capability — mirrors the WIT
/// `target.platforms-capability` record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlatformsCapability {
    /// Whether projects must declare a platform set.
    pub required: bool,
    /// The set of platforms this target accepts.
    pub allowed: Vec<Platform>,
    /// The set assumed when the operator accepts the default.
    pub default: Vec<Platform>,
}

/// A target adapter's metadata — mirrors the WIT `target.metadata`
/// record. Read by the host at resolve time from compiled-in constants.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TargetMetadata {
    /// Optional host-CLI compatibility floor (exact minimum `specify`
    /// version). Absent means no floor.
    pub specify_floor: Option<String>,
    /// Adapter-declared build inputs; empty when the target declares none.
    pub inputs: Vec<BuildInput>,
    /// Declarative platforms capability; absent when platform-agnostic.
    pub platforms: Option<PlatformsCapability>,
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
