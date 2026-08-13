//! Seam vocabulary mirroring the `emery:adapter` WIT records.
//!
//! Only answer-deserialized types carry serde derives.

use std::path::Path;

use omnia_guest::model::McpGrant;
use serde::Deserialize;

mod source;

pub use source::{
    Authority, Backing, Claim, ClaimKind, Evidence, Lead, SourceContent, SourceInput,
    SourceMetadata, SourceWorkspace, SurveyResult,
};

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
#[derive(Clone, Debug)]
pub struct Context<'a> {
    /// Adapter identity this call was routed by, e.g. `target:contracts`.
    pub adapter_id: &'a str,
    /// The guest's `"."` preopen root (the shared project mount).
    pub project_root: &'a Path,
    /// The adapter's MCP references endpoint, granted to the spawned
    /// agent so it can fetch `doc://` references lazily.
    pub mcp_url: Option<String>,
    /// Deployment-local path every judgment leg lends through
    /// `grants.workspace`. Defaults to the `"."` project mount
    /// (guidance); build and merge lend their prepared workspace via
    /// [`Self::lending`]. Source legs lend the CID view or [`None`]
    /// for an inline value — never the change home.
    pub lend: Option<String>,
}

impl<'a> Context<'a> {
    /// Guest-side context rooted at the guest's own `"."` preopen,
    /// granting the adapter's own references shelf ([`mcp_url`]) when
    /// the runtime injected an `HTTP_ADDR`.
    #[must_use]
    pub fn guest(adapter_id: &'a str) -> Self {
        Self {
            adapter_id,
            project_root: Path::new("."),
            mcp_url: mcp_url(adapter_id),
            lend: Some(".".to_string()),
        }
    }

    /// Lend `path` (a deployment-local directory, e.g. a prepared
    /// workspace root) to this context's judgment legs instead of the
    /// `"."` project mount.
    #[must_use]
    pub fn lending(mut self, path: impl Into<String>) -> Self {
        self.lend = Some(path.into());
        self
    }

    /// Issue the judgment call with no workspace lend (inline `value`).
    #[must_use]
    pub fn without_lend(mut self) -> Self {
        self.lend = None;
        self
    }

    /// MCP grants offered on every judgment leg: the adapter's own
    /// references, when an endpoint is set. Named `<name>-references`
    /// after the axis- and version-stripped adapter id, so a pinned
    /// dispatch (`target:omnia@1.0.0`) grants the same server name as
    /// an unpinned one.
    #[must_use]
    pub fn grants(&self) -> Vec<McpGrant> {
        let name = self.adapter_id.rsplit(':').next().unwrap_or(self.adapter_id);
        let name = name.split_once('@').map_or(name, |(stem, _)| stem);
        self.mcp_url
            .as_deref()
            .map(|url| McpGrant {
                name: format!("{name}-references"),
                tools: Vec::new(),
                url: url.to_string(),
            })
            .into_iter()
            .collect()
    }
}

/// The adapter's own MCP references endpoint on the runtime's HTTP
/// trigger: `http://127.0.0.1:<port>/mcp/<axis>/<name>[@<version>]`.
///
/// The port comes from the guest's `HTTP_ADDR` — injected by the
/// runtime from the local address of the deployment's pre-bound
/// listener, so grants and listener cannot drift apart. `None` when
/// the variable is absent or unparseable: no listener means no shelf,
/// and no grant is offered — degradation is coherent end to end,
/// never a wrong-port guess.
#[must_use]
pub fn mcp_url(adapter_id: &str) -> Option<String> {
    mcp_url_for(std::env::var("HTTP_ADDR").ok().as_deref(), adapter_id)
}

/// [`mcp_url`] over an explicit trigger address.
///
/// The path mirrors the routed adapter id verbatim (`:` becomes `/`,
/// the version pin stays), so the deployment's `http_paths` hook maps
/// it back onto the exact identity this guest was faulted in under and
/// the component's own `wasi:http` handler serves the shelf. Only the
/// port is taken from `addr`: the host stays the `IPv4` loopback
/// literal, not `localhost` — an agent whose resolver prefers `::1`
/// would otherwise fail to connect.
#[must_use]
pub fn mcp_url_for(addr: Option<&str>, adapter_id: &str) -> Option<String> {
    let port = addr?.rsplit_once(':')?.1.parse::<u16>().ok()?;
    Some(format!("http://127.0.0.1:{port}/mcp/{}", adapter_id.replacen(':', "/", 1)))
}

/// One slice-artifact payload — mirrors the WIT `payload` variant.
///
/// `Path` is the artifact's project-relative location ('/'-separated),
/// resolvable in the guest's `"."` preopen — never host-absolute;
/// prompts reference it from inside a lent workspace through
/// [`Workspace::artifact_path`]. `Body` is the inlined artifact text
/// for non-lent deployments (RFC-55). The cases are exclusive: the
/// engine sends `Path` while every deployment lends a workspace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Payload {
    /// Project-relative artifact path ('/'-separated).
    Path(String),
    /// Inlined artifact text when the deployment does not lend the tree.
    Body(String),
}

/// One slice-artifact input — mirrors the WIT `target.input` variant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Input {
    /// The slice's `proposal.md`.
    Proposal(Payload),
    /// The slice's `design.md`.
    Design(Payload),
    /// The slice's `tasks.md`.
    Tasks(Payload),
    /// One behavioural spec (`specs/<domain>/spec.md`).
    Spec(Payload),
    /// Any additional artifact.
    Other(Payload),
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

    /// The input's payload variant.
    #[must_use]
    pub const fn payload(&self) -> &Payload {
        match self {
            Self::Proposal(payload)
            | Self::Design(payload)
            | Self::Tasks(payload)
            | Self::Spec(payload)
            | Self::Other(payload) => payload,
        }
    }

    /// The input's project-relative artifact path, when path-form.
    #[must_use]
    pub const fn path(&self) -> Option<&str> {
        match self.payload() {
            Payload::Path(path) => Some(path.as_str()),
            Payload::Body(_) => None,
        }
    }

    /// The input's inlined artifact text, when body-form.
    #[must_use]
    pub const fn body(&self) -> Option<&str> {
        match self.payload() {
            Payload::Body(body) => Some(body.as_str()),
            Payload::Path(_) => None,
        }
    }
}

/// Deterministic per-slice facts the engine forwards to a build —
/// mirrors the WIT `target.build-context` record.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BuildContext {
    /// Kebab-case adapter names of the slice's bound sources. Empty
    /// when the slice has no resolvable plan entry.
    pub sources: Vec<String>,
}

/// The attempt-local writable artifact stage — mirrors the WIT
/// `target.artifact-stage` record (RFC-90 D5).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactStage {
    /// Opaque identity of the stage preparation.
    pub id: String,
    /// Deployment-local path of the writable stage root.
    pub root: String,
}

impl ArtifactStage {
    /// The stage root as a path, for in-guest filesystem access.
    #[must_use]
    pub fn root_path(&self) -> &Path {
        Path::new(&self.root)
    }
}

/// The private workspace an operation works on — mirrors the WIT
/// `target.workspace` record (RFC-87).
///
/// `root` is the deployment-local path of the writable code tree: the
/// adapter reads and writes product code there and lends it to its
/// agent by path. Change-tree artifacts stay outside the workspace,
/// readable through the `"."` preopen or the agent-visible
/// `artifacts` root; target-owned slice-artifact writes go to the
/// writable `artifact_stage` (present on build-loop operations).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Workspace {
    /// Opaque identity of the preparation.
    pub id: String,
    /// Deployment-local path of the writable workspace root.
    pub root: String,
    /// Agent-visible read-only artifact root (the project tree).
    pub artifacts: String,
    /// Writable artifact stage for the active slice; absent on
    /// `merge`, whose workspace view is read-only.
    pub artifact_stage: Option<ArtifactStage>,
}

impl Workspace {
    /// The workspace root as a path, for in-guest filesystem access.
    #[must_use]
    pub fn root_path(&self) -> &Path {
        Path::new(&self.root)
    }

    /// The agent-visible location of a project-relative artifact path
    /// — how prompts reference change-tree inputs from inside the
    /// lent workspace.
    #[must_use]
    pub fn artifact_path(&self, relative: &str) -> String {
        format!("{}/{relative}", self.artifacts)
    }
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

/// Judgment returned by `merge` — mirrors the WIT `report` record.
///
/// The resulting state lives in the working tree, not here. The
/// coverage claim rides the build phase report (RFC-86a D4), never
/// the merge return.
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

/// Adapter-selected outcome of one build phase — mirrors the WIT
/// `target.phase-outcome` enum (RFC-90 D2).
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum PhaseOutcome {
    /// The operation ran and produced its result.
    Completed,
    /// No target-specific work for this dispatch. Must carry no
    /// blocking findings and no writes.
    NotApplicable,
}

/// Report-level assurance claim — mirrors the WIT
/// `target.phase-source` enum. `Tool` is reserved on the wire but
/// rejected by the RFC-90 engine gate.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum PhaseSource {
    /// No model or external tool contributed.
    Deterministic,
    /// Model judgment produced the result, including agent-invoked
    /// native commands.
    ModelAssisted,
    /// More than one assurance source contributed.
    Hybrid,
    /// Trusted host-tool output. Reserved; rejected in RFC-90.
    Tool,
}

/// Which engine gate supplied a repair's findings — mirrors the WIT
/// `target.repair-origin` enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RepairOrigin {
    /// Findings from the latest verification report.
    Verification,
    /// Findings from the latest standards-review report.
    Review,
}

impl RepairOrigin {
    /// Kebab-case wire spelling, for prompt rendering.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Verification => "verification",
            Self::Review => "review",
        }
    }
}

/// Which writable root a phase write landed under — mirrors the WIT
/// `target.phase-root` enum.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum PhaseRoot {
    /// The writable product workspace.
    Workspace,
    /// The writable artifact stage.
    Artifacts,
}

/// One audit-evidence write reported by a phase — mirrors the WIT
/// `target.phase-write` record.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct PhaseWrite {
    /// Root the path is relative to.
    pub root: PhaseRoot,
    /// Root-relative '/'-separated path.
    pub path: String,
}

/// Grant grammar for one writable slice artifact — mirrors the WIT
/// `target.writable-artifact-kind` enum (RFC-90 D5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WritableArtifactKind {
    /// Exactly one slice-relative file.
    File,
    /// A directory and its descendants.
    Tree,
}

/// One target-declared writable slice artifact — mirrors the WIT
/// `target.writable-artifact` record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WritableArtifact {
    /// Slice-relative path of the granted file or tree root.
    pub path: String,
    /// File or tree grant.
    pub kind: WritableArtifactKind,
}

impl WritableArtifact {
    /// A `file` grant for one slice-relative path.
    #[must_use]
    pub fn file(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            kind: WritableArtifactKind::File,
        }
    }

    /// A `tree` grant for a slice-relative directory and its
    /// descendants.
    #[must_use]
    pub fn tree(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            kind: WritableArtifactKind::Tree,
        }
    }
}

/// Location anchor for a phase finding — mirrors the WIT
/// `target.phase-location` record.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct PhaseLocation {
    /// Project-relative file path.
    pub path: String,
    /// Anchor line.
    #[serde(default)]
    pub line: Option<u32>,
    /// Anchor column.
    #[serde(default)]
    pub column: Option<u32>,
    /// Inclusive end line for a range.
    #[serde(default)]
    pub end_line: Option<u32>,
    /// Inclusive end column for a range.
    #[serde(default)]
    pub end_column: Option<u32>,
}

/// Producer attribution of one finding — mirrors the WIT
/// `target.diagnostic-source` enum (the full `Diagnostic` source axis).
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum DiagnosticSource {
    /// Output of a deterministic scanner.
    Deterministic,
    /// Output of a model scorer.
    ModelAssisted,
    /// Mix of deterministic + model-assisted signals.
    Hybrid,
    /// Recorded by a human reviewer.
    Human,
    /// Emitted by an external tool.
    Tool,
}

/// Nature axis of one finding — mirrors the WIT `target.finding-kind`
/// enum. Only `violation` findings block.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum FindingKind {
    /// A defect: something is wrong and should be fixed.
    #[default]
    Violation,
    /// A deterministically raised request for judgment.
    Review,
}

/// Artifact category attribution — mirrors the WIT
/// `target.finding-artifact` enum.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum FindingArtifact {
    /// Generated or hand-written code.
    Code,
    /// Test files.
    Tests,
    /// Contract artifacts.
    Contracts,
    /// Behavioral specs.
    Specs,
    /// Design notes.
    Design,
    /// Decision Records.
    Decisions,
    /// Task list.
    Tasks,
    /// Asset inventory.
    Assets,
    /// Design tokens.
    Tokens,
    /// Per-shell composition manifest.
    Composition,
    /// Plan or workflow artifact.
    Plan,
    /// Not classified.
    Unknown,
}

/// Producer self-rated confidence — mirrors the WIT
/// `target.finding-confidence` enum.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum FindingConfidence {
    /// High confidence.
    High,
    /// Medium confidence.
    Medium,
    /// Low confidence; reviewer should triage.
    Low,
}

/// The closed evidence union of a phase finding — mirrors the WIT
/// `target.finding-evidence` variant. Internally tagged on `kind` to
/// match the `Diagnostic` wire shape.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum FindingEvidence {
    /// Bounded verbatim excerpt.
    Snippet {
        /// Verbatim payload.
        value: String,
    },
    /// Digest reference for evidence too large or sensitive to inline.
    Digest {
        /// Hex-encoded SHA-256 of the underlying evidence bytes.
        sha256: String,
        /// Short human summary of what was hashed.
        summary: String,
        /// Optional contributing locations.
        #[serde(default)]
        locations: Option<Vec<PhaseLocation>>,
    },
    /// Domain-structured evidence.
    Structured {
        /// Short human summary of `data`.
        summary: String,
        /// Free-form JSON payload.
        data: serde_json::Value,
        /// Optional contributing locations.
        #[serde(default)]
        locations: Option<Vec<PhaseLocation>>,
    },
}

/// One phase finding — the isomorphic mirror of the shared
/// `Diagnostic` wire shape (RFC-90 D2).
///
/// The engine stamps identity fields, recomputes the fingerprint, and
/// renumbers report-local ids; adapters never fold title / impact /
/// remediation prose.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub struct PhaseFinding {
    /// Report-local stable id (renumbered by the engine).
    #[serde(default)]
    pub id: String,
    /// Codex rule citation, if any.
    #[serde(default)]
    pub rule_id: Option<String>,
    /// Additional codex ids that informed the finding.
    ///
    /// The phase-report answer schema (from `diagnostics::Diagnostic`)
    /// admits `null`; treat that the same as omission / `[]`.
    #[serde(default, deserialize_with = "null_as_default")]
    pub related_rule_ids: Vec<String>,
    /// Short finding title.
    pub title: String,
    /// Review severity.
    pub severity: Severity,
    /// Producer attribution.
    pub source: DiagnosticSource,
    /// Defect vs request-for-judgment.
    #[serde(default)]
    pub kind: FindingKind,
    /// Artifact category attribution.
    pub artifact: FindingArtifact,
    /// Optional anchor location.
    #[serde(default)]
    pub location: Option<PhaseLocation>,
    /// Evidence union.
    pub evidence: FindingEvidence,
    /// Operator-facing risk.
    pub impact: String,
    /// Concrete action to clear the finding.
    pub remediation: String,
    /// Producer self-rated confidence.
    #[serde(default)]
    pub confidence: Option<FindingConfidence>,
    /// Stable `sha256:<64 hex>` hash; recomputed by the engine.
    #[serde(default)]
    pub fingerprint: String,
}

impl PhaseFinding {
    /// Whether this finding blocks (an important-or-worse violation).
    #[must_use]
    pub const fn blocking(&self) -> bool {
        matches!(self.kind, FindingKind::Violation) && self.severity.blocking()
    }
}

/// Deserialize a field that the answer schema admits as `T | null`
/// into `T`, treating `null` as [`Default::default`].
fn null_as_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Default + Deserialize<'de>,
{
    Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
}

/// The typed result of exactly one build-phase operation — mirrors
/// the WIT `target.phase-report` record (RFC-90 D2). `outputs`,
/// `ui_surface`, and `covered` are meaningful only on `build` reports.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PhaseReport {
    /// Adapter-selected outcome.
    pub outcome: PhaseOutcome,
    /// Required report-level assurance claim.
    pub source: PhaseSource,
    /// Phase findings.
    pub findings: Vec<PhaseFinding>,
    /// Candidate per-platform outputs (`build` only).
    pub outputs: Vec<BuildOutput>,
    /// Candidate UI-surface signal (`build` only).
    pub ui_surface: Option<UiSurface>,
    /// Slice-local requirement ids the phase claims to have
    /// implemented (`build` only). Must never name a requirement from
    /// the build request's `deferred[]` exclusion set (RFC-86a D4).
    pub covered: Vec<String>,
    /// Audit-evidence writes performed by the phase.
    pub written: Vec<PhaseWrite>,
    /// Adapter-opaque continuation: `None` preserves, `Some(vec![])`
    /// clears, non-empty replaces. `verify` cannot mutate it.
    pub next_continuation: Option<Vec<u8>>,
}

impl PhaseReport {
    /// A completed report with the given assurance source and no
    /// findings, writes, outputs, or continuation change.
    #[must_use]
    pub const fn completed(source: PhaseSource) -> Self {
        Self {
            outcome: PhaseOutcome::Completed,
            source,
            findings: Vec::new(),
            outputs: Vec::new(),
            ui_surface: None,
            covered: Vec::new(),
            written: Vec::new(),
            next_continuation: None,
        }
    }

    /// A typed non-applicable report: deterministic, no findings, no
    /// writes — the shape an adapter returns when an operation has no
    /// target-specific work (RFC-90 D7).
    #[must_use]
    pub const fn not_applicable() -> Self {
        Self {
            outcome: PhaseOutcome::NotApplicable,
            source: PhaseSource::Deterministic,
            findings: Vec::new(),
            outputs: Vec::new(),
            ui_surface: None,
            covered: Vec::new(),
            written: Vec::new(),
            next_continuation: None,
        }
    }
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
    /// Optional host-CLI compatibility floor (exact minimum `emery`
    /// version). Absent means no floor.
    pub emery_floor: Option<String>,
    /// Adapter-declared build inputs; empty when the target declares none.
    pub inputs: Vec<BuildInput>,
    /// Declarative platforms capability; absent when platform-agnostic.
    pub platforms: Option<PlatformsCapability>,
    /// Typed writable slice-artifact grants (RFC-90 D5); empty when
    /// the target writes no slice artifacts.
    pub writable_artifacts: Vec<WritableArtifact>,
}
