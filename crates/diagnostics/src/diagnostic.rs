//! Neutral diagnostics and their closed attributes.
//!
//! [`DiagnosticSource`] (*who produced it*) and [`DiagnosticKind`] (*what it
//! asks*) are orthogonal; only `violation` diagnostics are default-blocking.

use serde::{Deserialize, Serialize};

mod report;

pub use report::{DiagnosticReport, DiagnosticReportVersion, DiagnosticSummary};

/// Severity in ascending derived sort order: critical to optional.
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
    schemars::JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum Severity {
    /// Highest priority; blocks merge in CI.
    Critical,
    /// Should-fix; default escalation level for adapter overlays.
    Important,
    /// Nice-to-have; reviewer judgement applies.
    Suggestion,
    /// Informational; recorded but not graded.
    Optional,
}

/// Producer attribution for a [`Diagnostic`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum DiagnosticSource {
    /// Output of a deterministic scanner.
    Deterministic,
    /// Output of an SLM/LLM scorer.
    ModelAssisted,
    /// Mix of deterministic and model-assisted signals.
    Hybrid,
    /// Recorded by a human reviewer.
    Human,
    /// Emitted by an external WASI tool (e.g. the contract verifier).
    Tool,
}

/// Defect versus request-for-judgment axis.
///
/// Missing wire values default to [`Self::Violation`].
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum DiagnosticKind {
    /// A defect; the default.
    #[default]
    Violation,
    /// A request for agent or human judgment.
    Review,
}

/// Artifact category attribution for a [`Diagnostic`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Artifact {
    /// Generated or hand-written code.
    Code,
    /// Test files.
    Tests,
    /// Contract artifacts under `contracts/`.
    Contracts,
    /// Behavioral specs (`spec.md`).
    Specs,
    /// Design notes (`design.md`).
    Design,
    /// Decision records.
    Decisions,
    /// Task list (`tasks.md`).
    Tasks,
    /// Asset inventory (`assets.yaml`).
    Assets,
    /// Design tokens (`tokens.yaml`).
    Tokens,
    /// Per-shell composition manifest.
    Composition,
    /// Plan or workflow artifact (`plan.yaml`, `leads.md`).
    Plan,
    /// Artifact category not classified.
    Unknown,
}

/// Producer self-rated confidence for a [`Diagnostic`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Confidence {
    /// High confidence in the diagnostic.
    High,
    /// Medium confidence.
    Medium,
    /// Low confidence; reviewer should triage.
    Low,
}

/// File path and optional line/column range.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct FindingLocation {
    /// Project-relative file path.
    pub path: String,
    /// Anchor line; schema minimum is zero.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    /// Anchor column.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column: Option<u32>,
    /// Inclusive end line for a multi-line range.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_line: Option<u32>,
    /// Inclusive end column for a multi-line range.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_column: Option<u32>,
}

/// Closed, `kind`-tagged diagnostic evidence.
///
/// Keep inline payloads small; reference large evidence by digest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum FindingEvidence {
    /// Bounded verbatim excerpt for local code or prose evidence.
    Snippet {
        /// Verbatim payload bytes.
        value: String,
    },
    /// Digest reference for evidence too large or sensitive to inline.
    Digest {
        /// Hex-encoded SHA-256 of the underlying evidence bytes.
        sha256: String,
        /// Short human summary of what was hashed.
        summary: String,
        /// Optional contributing locations referenced by the digest.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        locations: Option<Vec<FindingLocation>>,
    },
    /// Domain-structured evidence.
    Structured {
        /// Short human summary of `data`.
        summary: String,
        /// Secret-free JSON; the full evidence object is capped at 16 `KiB`.
        data: serde_json::Value,
        /// Optional contributing locations referenced by the payload.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        locations: Option<Vec<FindingLocation>>,
    },
}

/// Structured diagnostic shared by validation and review.
///
/// Producer-local `id` (e.g. `FIND-0001`) is distinct from the codex
/// `rule_id` (e.g. `UNI-014`): `id` is a stable per-run handle and
/// `rule_id` is the durable codex citation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Diagnostic {
    /// Producer-local stable id for this run.
    pub id: String,
    /// Cited rule id, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rule_id: Option<String>,
    /// Additional codex ids that informed the diagnostic.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub related_rule_ids: Option<Vec<String>>,
    /// Short diagnostic title.
    pub title: String,
    /// Closed severity enum.
    pub severity: Severity,
    /// Producer attribution.
    pub source: DiagnosticSource,
    /// Defect or review request; defaults to `violation`.
    #[serde(default)]
    pub kind: DiagnosticKind,
    /// Target-adapter name when the diagnostic is adapter-specific.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_adapter: Option<String>,
    /// Source-adapter name when the diagnostic is source-specific.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_adapter: Option<String>,
    /// Slice name when the diagnostic is slice-scoped.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slice: Option<String>,
    /// Change name when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub change: Option<String>,
    /// Artifact category attribution.
    pub artifact: Artifact,
    /// Optional anchor location for the diagnostic.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<FindingLocation>,
    /// Evidence union.
    pub evidence: FindingEvidence,
    /// Operator-facing risk.
    pub impact: String,
    /// Concrete action to clear the diagnostic.
    pub remediation: String,
    /// Producer confidence; required for model-assisted sources by validation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<Confidence>,
    /// Stable hash over `(rule-id, location, evidence-payload)`.
    /// Format `sha256:<64 hex chars>`.
    pub fingerprint: String,
}

impl Diagnostic {
    /// Build a finding with a computed fingerprint.
    ///
    /// `detail` supplies evidence and impact; `title` supplies remediation.
    /// [`renumber`] replaces the placeholder id.
    #[expect(
        clippy::too_many_arguments,
        reason = "eight independent finding facets with no natural grouping; \
                  the violation/review shortcuts cover the common shapes"
    )]
    #[must_use]
    pub fn finding(
        rule_id: impl Into<String>, title: impl Into<String>, detail: impl Into<String>,
        severity: Severity, kind: DiagnosticKind, source: DiagnosticSource, artifact: Artifact,
        location: Option<FindingLocation>,
    ) -> Self {
        let title = non_empty(title.into(), "finding");
        let detail = non_empty(detail.into(), &title);
        let confidence =
            matches!(source, DiagnosticSource::ModelAssisted | DiagnosticSource::Hybrid)
                .then_some(Confidence::Medium);
        let mut diagnostic = Self {
            id: "DIAG-0001".to_string(),
            rule_id: Some(rule_id.into()),
            related_rule_ids: None,
            title: title.clone(),
            severity,
            source,
            kind,
            target_adapter: None,
            source_adapter: None,
            slice: None,
            change: None,
            artifact,
            location,
            evidence: FindingEvidence::Snippet {
                value: detail.clone(),
            },
            impact: detail,
            remediation: title,
            confidence,
            fingerprint: String::new(),
        };
        diagnostic.fingerprint = crate::fingerprint::fingerprint(&diagnostic);
        diagnostic
    }

    /// Build a deterministic, important violation.
    #[must_use]
    pub fn violation(
        rule_id: impl Into<String>, title: impl Into<String>, detail: impl Into<String>,
        artifact: Artifact, location: Option<FindingLocation>,
    ) -> Self {
        Self::finding(
            rule_id,
            title,
            detail,
            Severity::Important,
            DiagnosticKind::Violation,
            DiagnosticSource::Deterministic,
            artifact,
            location,
        )
    }

    /// Build a model-assisted, non-blocking review suggestion.
    #[must_use]
    pub fn review(
        rule_id: impl Into<String>, title: impl Into<String>, detail: impl Into<String>,
        artifact: Artifact, location: Option<FindingLocation>,
    ) -> Self {
        Self::finding(
            rule_id,
            title,
            detail,
            Severity::Suggestion,
            DiagnosticKind::Review,
            DiagnosticSource::ModelAssisted,
            artifact,
            location,
        )
    }
}

// Preserve schema `minLength: 1` fields.
fn non_empty(value: String, fallback: &str) -> String {
    if value.trim().is_empty() { fallback.to_string() } else { value }
}

/// Assign sequential `DIAG-NNNN` ids in final order.
///
/// Ids are excluded from fingerprints, so renumbering preserves identity.
pub fn renumber(findings: &mut [Diagnostic]) {
    for (index, finding) in findings.iter_mut().enumerate() {
        finding.id = format!("DIAG-{:04}", index + 1);
    }
}

/// Whether a critical or important violation blocks exit.
#[must_use]
pub const fn is_blocking(diagnostic: &Diagnostic) -> bool {
    matches!(diagnostic.kind, DiagnosticKind::Violation)
        && matches!(diagnostic.severity, Severity::Critical | Severity::Important)
}

/// Whether any diagnostic in `diagnostics` blocks per [`is_blocking`].
#[must_use]
pub fn has_blocking(diagnostics: &[Diagnostic]) -> bool {
    diagnostics.iter().any(is_blocking)
}
