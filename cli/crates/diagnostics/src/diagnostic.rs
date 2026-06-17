//! The neutral [`Diagnostic`] currency and its closed attribute enums.
//!
//! A [`Diagnostic`] is the single structured finding shape shared by
//! both Specify surfaces: the advisory `lint` surface (`specify lint`,
//! `specify lint framework`, target-adapter review briefs, model-assisted
//! scorers, CI annotations) and the workflow-gating `validate`
//! surface (slice/plan structural invariants). The two surfaces stay
//! conceptually distinct — they differ in gate policy, not in
//! currency — so the substrate is named neutrally rather than after
//! either surface.
//!
//! Two orthogonal axes classify a diagnostic:
//!
//! - [`DiagnosticSource`] — *who produced it* (`deterministic`,
//!   `model-assisted`, `hybrid`, `human`, `tool`).
//! - [`DiagnosticKind`] — *what it asks of the reader*: a
//!   [`DiagnosticKind::Violation`] is a defect to fix; a
//!   [`DiagnosticKind::Review`] is a deterministically-raised request
//!   for agent or human judgment (e.g. a deferred semantic check).
//!   Only `violation` diagnostics are default-blocking — see
//!   [`blocking`].
//!
//! Field names are kebab-case at every nesting level. Producer-local
//! `id` (e.g. `FIND-0001`) is distinct from the codex `rule_id` (e.g.
//! `UNI-014`): `id` is a stable per-run handle and `rule_id` is the
//! durable codex citation.
//!
//! Severity comparator order is `Critical < Important < Suggestion <
//! Optional`; the closed enum is declared in that order so the derived
//! [`Ord`] picks up the contract-defined sort sequence.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Closed severity enum. Variants are declared in the documented sort
/// order — the derived [`Ord`] therefore yields `Critical < Important
/// < Suggestion < Optional`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DiagnosticSource {
    /// Output of a deterministic scanner.
    Deterministic,
    /// Output of an SLM/LLM scorer.
    ModelAssisted,
    /// Mix of deterministic + model-assisted signals.
    Hybrid,
    /// Recorded by a human reviewer.
    Human,
    /// Emitted by an external WASI tool (e.g. the contract verifier).
    Tool,
}

/// Orthogonal nature axis for a [`Diagnostic`].
///
/// Distinguishes a deterministic defect from a deterministically
/// raised request for judgment. Defaults to [`Self::Violation`] so a
/// diagnostic that omits the wire field deserialises as a defect, and
/// so the [`blocking`] predicate keeps its pre-axis behaviour.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DiagnosticKind {
    /// A defect: something is wrong and should be fixed. Default.
    #[default]
    Violation,
    /// A request for agent or human judgment raised deterministically
    /// (e.g. a deferred semantic check the producer cannot decide).
    Review,
}

/// Artifact category attribution for a [`Diagnostic`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
    /// Decision Records (`.specify/slices/<slice>/decisions/<slug>.md`,
    /// promoted to `.specify/decisions/DEC-NNNN-<slug>.md`).
    Decisions,
    /// Task list (`tasks.md`).
    Tasks,
    /// Asset inventory (`assets.yaml`).
    Assets,
    /// Design tokens (`tokens.yaml`).
    Tokens,
    /// Per-shell composition manifest.
    Composition,
    /// Plan or workflow artifact (`plan.yaml`, `discovery.md`).
    Plan,
    /// Artifact category not classified.
    Unknown,
}

/// Producer self-rated confidence for a [`Diagnostic`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Confidence {
    /// High confidence in the diagnostic.
    High,
    /// Medium confidence.
    Medium,
    /// Low confidence; reviewer should triage.
    Low,
}

/// Triage status for a [`Diagnostic`]. Omitted by raw scanners and
/// populated by review reports, the directive post-pass, or
/// CI state.
///
/// `Ignored` is set by the directive pass when a `specify-ignore`
/// directive matches a diagnostic; `FalsePositive` is set by the same
/// pass when the directive's rationale begins with `false-positive:`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FindingStatus {
    /// Untriaged; default for fresh diagnostics and the only
    /// default-blocking value at exit time.
    Open,
    /// Demoted by a matching `specify-ignore` directive.
    Ignored,
    /// Resolved by a code change.
    Fixed,
    /// Operator-acknowledged; will not be fixed.
    Accepted,
    /// Producer-mistaken; the diagnostic does not apply.
    FalsePositive,
}

/// Origin of a non-`open` `status` on a [`Diagnostic`].
///
/// Closed discriminator for the `disposition.source` wire field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DispositionSource {
    /// `specify-ignore` directive in the scanned source.
    Directive,
}

/// `disposition.directive` payload populated when
/// [`FindingDisposition::source`] is [`DispositionSource::Directive`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct DirectiveDisposition {
    /// Project-relative path of the source file containing the
    /// directive comment.
    pub path: String,
    /// 1-based line of the directive comment itself (not the target
    /// line the directive applies to).
    pub line: u32,
    /// Free-form rationale captured verbatim from the directive
    /// comment.
    pub rationale: String,
}

/// Origin of a non-`open` finding status on a [`Diagnostic`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct FindingDisposition {
    /// Closed discriminator naming the disposition's origin.
    pub source: DispositionSource,
    /// Directive payload, populated when `source` is
    /// [`DispositionSource::Directive`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub directive: Option<DirectiveDisposition>,
    /// Optional free-form marker indicating when the disposition took
    /// effect (commit hash, ISO-8601 timestamp, release tag, …).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub since: Option<String>,
}

/// File path plus optional line/column range carried by a
/// [`Diagnostic`] or by a `digest`/`structured` evidence variant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct FindingLocation {
    /// Project-relative file path.
    pub path: String,
    /// Anchor line (0-indexed; producers commonly emit 1-indexed and
    /// the schema accepts either with `minimum: 0`).
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

/// Closed evidence union for a [`Diagnostic`].
///
/// Internally tagged on `kind`; the wire shape's `oneOf` is encoded
/// by serde's `tag = "kind"` with `additionalProperties: false` per
/// branch validated schema-side. The diagnostic contract caps the
/// serialized evidence payload at 16 `KiB`, enforced by
/// [`crate::validate::validate_evidence_size`], not here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    /// Domain-structured evidence (e.g. contract compatibility data).
    Structured {
        /// Short human summary of `data`.
        summary: String,
        /// Free-form JSON payload. Producers MUST keep `data` bounded
        /// and secret-free; the validator enforces the 16 `KiB` cap on
        /// the full evidence object.
        data: serde_json::Value,
        /// Optional contributing locations referenced by the payload.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        locations: Option<Vec<FindingLocation>>,
    },
}

/// Structured diagnostic — the neutral currency shared by the lint and
/// validate surfaces.
///
/// Producer-local `id` (e.g. `FIND-0001`) is distinct from the codex
/// `rule_id` (e.g. `UNI-014`): `id` is a stable per-run handle and
/// `rule_id` is the durable codex citation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Diagnostic {
    /// Producer-local stable id for this run (e.g. `FIND-0001`).
    pub id: String,
    /// Rule id (e.g. `UNI-014`); absent for diagnostics that do not
    /// cite codex policy.
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
    /// Nature axis: defect (`violation`) vs request-for-judgment
    /// (`review`). Defaults to `violation` when the wire field is
    /// omitted.
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
    /// Producer self-rated confidence. Required for
    /// `source: model-assisted`; the conditional rule is enforced by
    /// the validator, not here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<Confidence>,
    /// Stable hash over `(rule-id, location, evidence-payload)`.
    /// Format `sha256:<64 hex chars>`.
    pub fingerprint: String,
    /// Triage status. Omitted by raw scanners; populated by review
    /// reports, the directive post-pass, or CI state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<FindingStatus>,
    /// Origin of a non-`open` `status`. Unset when `status` is `open`
    /// or absent. Excluded from the fingerprint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disposition: Option<FindingDisposition>,
}

impl Diagnostic {
    /// Build a workflow/validate finding with a computed fingerprint.
    ///
    /// `rule_id` is the dot- or kebab-namespaced invariant id (e.g.
    /// `spec.requirement-id-missing`, `slice-model-source-orphan`); the
    /// finding schema's `ruleId` pattern accepts this namespace
    /// alongside the codex `UNI-`/`CORE-` family. `detail` becomes both
    /// the snippet evidence payload and the operator-facing `impact`;
    /// `title` doubles as the `remediation` (it states the invariant the
    /// producer expects to hold). The `id` is a placeholder until
    /// [`renumber`] assigns sequential ids at report-assembly time.
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
            status: None,
            disposition: None,
        };
        diagnostic.fingerprint = crate::fingerprint::fingerprint(&diagnostic);
        diagnostic
    }

    /// Deterministic, `important`, [`DiagnosticKind::Violation`] finding —
    /// the default shape for a structural workflow invariant breach.
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

    /// Model-assisted, `suggestion`, [`DiagnosticKind::Review`] finding —
    /// a deterministically-raised request for agent/human judgment (e.g.
    /// a deferred semantic check). Never default-blocking.
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

/// Substitute `fallback` when `value` is blank so schema `minLength: 1`
/// fields (`title`, `impact`, `remediation`, snippet `value`) never go
/// empty.
fn non_empty(value: String, fallback: &str) -> String {
    if value.trim().is_empty() { fallback.to_string() } else { value }
}

/// Assign sequential `DIAG-NNNN` ids to `findings` in place.
///
/// Producers build findings with a placeholder `id`; the handler that
/// assembles the [`DiagnosticReport`] calls this once the final,
/// deduplicated order is known so the rendered ids are stable and
/// unique. The `id` is excluded from the fingerprint, so renumbering
/// never perturbs dedup identity.
pub fn renumber(findings: &mut [Diagnostic]) {
    for (index, finding) in findings.iter_mut().enumerate() {
        finding.id = format!("DIAG-{:04}", index + 1);
    }
}

/// Whether a diagnostic blocks at exit time.
///
/// A diagnostic blocks only when it is a [`DiagnosticKind::Violation`]
/// (a `review` request never gates), its severity is the blocking tier
/// (`critical` or `important`), and its status is untriaged (`open` or
/// unset). Demoted statuses (`ignored`, `accepted`, `fixed`,
/// `false-positive`) never block.
#[must_use]
pub const fn blocking(diagnostic: &Diagnostic) -> bool {
    matches!(diagnostic.kind, DiagnosticKind::Violation)
        && matches!(diagnostic.severity, Severity::Critical | Severity::Important)
        && matches!(diagnostic.status, None | Some(FindingStatus::Open))
}

/// Whether any diagnostic in `diagnostics` blocks per [`blocking`].
#[must_use]
pub fn blocking_present(diagnostics: &[Diagnostic]) -> bool {
    diagnostics.iter().any(blocking)
}

/// Type-level pin of the [`DiagnosticReport`] envelope version.
///
/// Serialises to the integer `1` and refuses to deserialise any other
/// value.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DiagnosticReportVersion;

impl Serialize for DiagnosticReportVersion {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u32(1)
    }
}

impl<'de> Deserialize<'de> for DiagnosticReportVersion {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = u32::deserialize(deserializer)?;
        if value == 1 {
            Ok(Self)
        } else {
            Err(serde::de::Error::custom(format!(
                "unsupported DiagnosticReport version: {value} (only v1 is supported)"
            )))
        }
    }
}

/// Diagnostic tally by severity for the [`DiagnosticReport`] envelope.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticSummary {
    /// Count of diagnostics with `severity: critical`.
    pub critical: u32,
    /// Count of diagnostics with `severity: important`.
    pub important: u32,
    /// Count of diagnostics with `severity: suggestion`.
    pub suggestion: u32,
    /// Count of diagnostics with `severity: optional`.
    pub optional: u32,
}

impl DiagnosticSummary {
    /// Tally `diagnostics` by severity.
    #[must_use]
    pub fn from_diagnostics(diagnostics: &[Diagnostic]) -> Self {
        let mut summary = Self::default();
        for diagnostic in diagnostics {
            match diagnostic.severity {
                Severity::Critical => summary.critical += 1,
                Severity::Important => summary.important += 1,
                Severity::Suggestion => summary.suggestion += 1,
                Severity::Optional => summary.optional += 1,
            }
        }
        summary
    }
}

/// Diagnostic report envelope — `{ version, summary, diagnostics }`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticReport {
    /// Envelope version discriminant pinned to `1`.
    pub version: DiagnosticReportVersion,
    /// Diagnostic tally by severity.
    pub summary: DiagnosticSummary,
    /// Byte-stable list of structured diagnostics. Ordering is the
    /// producer's responsibility; this crate preserves the input order
    /// on every formatter.
    pub findings: Vec<Diagnostic>,
}

/// Count diagnostics whose `status` matches `target`.
///
/// Passing `None` counts the `open` bucket — an unset `status` is
/// treated as `Open`.
#[must_use]
pub fn count_status(diagnostics: &[Diagnostic], target: Option<FindingStatus>) -> u32 {
    let count = diagnostics
        .iter()
        .filter(|d| {
            target.map_or_else(
                || matches!(d.status, None | Some(FindingStatus::Open)),
                |want| d.status == Some(want),
            )
        })
        .count();
    u32::try_from(count).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::{
        DiagnosticKind, DiagnosticReportVersion, DiagnosticSource, DiagnosticSummary, Severity,
        blocking,
    };
    use crate::test_support::diagnostic;

    #[test]
    fn version_serialises_as_one() {
        let v = serde_json::to_value(DiagnosticReportVersion).expect("serialise");
        assert_eq!(v, Value::from(1));
    }

    #[test]
    fn version_rejects_other_values() {
        let err = serde_json::from_value::<DiagnosticReportVersion>(Value::from(2))
            .expect_err("v2 must be rejected");
        assert!(err.to_string().contains("unsupported DiagnosticReport version"));
    }

    #[test]
    fn summary_counts_each_severity() {
        let findings = vec![
            diagnostic("a", Severity::Critical),
            diagnostic("b", Severity::Important),
            diagnostic("c", Severity::Important),
            diagnostic("d", Severity::Suggestion),
            diagnostic("e", Severity::Optional),
        ];
        let summary = DiagnosticSummary::from_diagnostics(&findings);
        assert_eq!(summary.critical, 1);
        assert_eq!(summary.important, 2);
        assert_eq!(summary.suggestion, 1);
        assert_eq!(summary.optional, 1);
    }

    /// A `review`-kind diagnostic never blocks even at a blocking
    /// severity; a `violation` at the same severity does.
    #[test]
    fn review_kind_never_blocks() {
        let mut review = diagnostic("r", Severity::Critical);
        review.kind = DiagnosticKind::Review;
        assert!(!blocking(&review), "review-kind must not block");

        let mut violation = diagnostic("v", Severity::Critical);
        violation.kind = DiagnosticKind::Violation;
        assert!(blocking(&violation), "violation at critical must block");
    }

    #[test]
    fn severity_ordering_matches_contract() {
        assert!(Severity::Critical < Severity::Important);
        assert!(Severity::Important < Severity::Suggestion);
        assert!(Severity::Suggestion < Severity::Optional);
    }

    /// `tool` is a legal producer source.
    #[test]
    fn tool_source_round_trips() {
        let value = serde_json::to_value(DiagnosticSource::Tool).expect("serialise");
        assert_eq!(value, Value::from("tool"));
    }
}
