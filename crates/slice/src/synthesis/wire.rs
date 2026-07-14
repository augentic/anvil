//! Synthesis response wire DTO + input-envelope assembly.
//!
//! The single schema-gated wire is the **response**:
//! [`SynthesisResponse`] is the source of truth — the judgment-answer
//! schema the model host gates against is generated from it
//! ([`crate::answers::synthesis`]) and the deterministic tail re-parses
//! the raw answer through it. The inputs [`inputs`] assembles into
//! [`SynthesisInputs`] have no closed request shape. Authority is not included — the kernel
//! resolves it from the on-disk Evidence after the response returns.
//! Assembly is pure over already-read inputs;
//! [`SourceInput::from_file`] is the only filesystem hook.

use std::path::Path;

use error::Result;
use project::registry::topology::{Decision, Surface};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::model::SliceModel;
use crate::synthesis::baseline::BaselineIndex;

/// Wire version stamped on both the input and response envelopes.
const SYNTHESIS_VERSION: u32 = 1;

/// Synthesis response envelope kind.
///
/// Serialises to the literal `"response"`. Mirrors
/// `project::plan::propose`'s
/// `ProposalKind`, but synthesis has only the response kind — there is
/// no request wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum SynthesisKind {
    /// The agent's synthesis result.
    Response,
}

/// `kind: response` envelope — the agent's synthesis result.
///
/// The DTO is shape-only and the source of the generated
/// judgment-answer schema; the projection kernel re-derives every
/// kernel-owned field after the parse.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SynthesisResponse {
    /// Schema version.
    pub version: u32,
    /// Envelope kind.
    pub kind: SynthesisKind,
    /// Slice name (kebab-case).
    pub slice: String,
    /// The agent's structured model — the kernel-owned and header
    /// fields are optional in [`SliceModel`], so the agent's
    /// kernel-omitted model deserialises cleanly.
    pub model: SliceModel,
    /// Prose-only Markdown artifacts (no `ID:` / `Sources:` / `Status:`
    /// lines — the render step injects those).
    pub artifacts: SynthesisArtifacts,
}

/// The prose-only Markdown artifacts under a [`SynthesisResponse`].
///
/// Each is authored by the agent; the render step later injects
/// provenance lines into the spec bodies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SynthesisArtifacts {
    /// `proposal.md` body.
    pub proposal: String,
    /// `design.md` body.
    pub design: String,
    /// `tasks.md` body.
    pub tasks: String,
    /// Per-domain spec bodies (`specs/<domain>/spec.md`).
    pub specs: Vec<SynthesisSpec>,
    /// Optional slice-authored Decision Records, rendered by the
    /// persist tail to `decisions/<slug>.md` with exact-set
    /// replacement. Empty means the slice sets no durable decision.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub decisions: Vec<SynthesisDecision>,
}

/// One slice-authored Decision Record under
/// [`SynthesisArtifacts::decisions`].
///
/// Carries only the agent-authored fields plus the Nygard prose; the
/// engine stamps `id` / `slice` / `date` (and any `superseded-by`) at
/// merge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SynthesisDecision {
    /// Stable kebab-case slug — the baseline filename derives from it.
    pub slug: String,
    /// `accepted` or `rejected` (`superseded` is engine-only and
    /// refused by the answer schema).
    pub status: artifacts::decision::DecisionStatus,
    /// The record's H1 title.
    pub title: String,
    /// `## Context` body.
    pub context: String,
    /// `## Decision` body.
    pub decision: String,
    /// `## Consequences` body.
    pub consequences: String,
    /// Records this decision supersedes — baseline `DEC-NNNN` ids or
    /// sibling slugs from this same slice.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub supersedes: Vec<String>,
    /// Optional traceability into this slice's requirements (`REQ-NNN`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related: Vec<String>,
    /// Optional kebab-case topic slugs the decision governs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub topics: Vec<String>,
}

/// One per-domain spec body under [`SynthesisArtifacts::specs`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SynthesisSpec {
    /// Owning domain (kebab-case spec group).
    pub domain: String,
    /// The spec body, without `ID:` / `Sources:` / `Status:` lines.
    pub content: String,
}

/// Synthesis input envelope kind.
///
/// The inputs are not schema-validated (there is no closed request
/// shape), but the envelope still carries a
/// closed discriminator for symmetry with the response.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum InputKind {
    /// Agent synthesis inputs.
    Inputs,
}

/// The agent synthesis step's input envelope.
///
/// Assembled by [`inputs`] for the guest refine
/// orchestration. Not schema-validated —
/// synthesis is always agent-dispatched, so there is no tool consumer
/// and no closed request schema. Authority is deliberately absent: the
/// kernel resolves it post-response from on-disk Evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct SynthesisInputs {
    /// Schema version.
    pub version: u32,
    /// Envelope kind.
    pub kind: InputKind,
    /// Slice name the step synthesises.
    pub slice: String,
    /// One entry per bound source, carrying its inline `lead` and
    /// `claims`.
    pub sources: Vec<SourceInput>,
    /// The resolved target guidance body. Resolved and read by the
    /// refine orchestration — never by this module.
    pub guidance_brief: String,
    /// The slice's bound project baseline surface (one entry
    /// per `.specify/specs/<domain>/spec.md`), so synthesis reconciles
    /// against existing requirements instead of duplicating them. Read
    /// from the project topology at assembly time; empty (greenfield, or
    /// no baseline) stays off the wire. Context only — no schema or
    /// `model.yaml` change.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub baseline: Vec<Surface>,
    /// Per-domain baseline `REQ` ids and the highest assigned suffix,
    /// advisory context for id assignment in modified domains. Empty
    /// stays off the wire (greenfield).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub baseline_detail: Vec<DomainDetail>,
    /// The bound project's accepted baseline Decision Records
    /// (`id` / `title` / `topics`), projected through the registry
    /// identity machinery. Gives the response's optional `decisions[]`
    /// valid `supersedes:` targets without filesystem discovery. Empty
    /// stays off the wire (no catalogue).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub baseline_decisions: Vec<Decision>,
}

/// Advisory per-domain baseline id facts for the synthesis inputs envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct DomainDetail {
    /// Domain directory slug under `.specify/specs/`.
    pub domain: String,
    /// Existing baseline `REQ-NNN` ids in id order.
    pub req_ids: Vec<String>,
    /// Highest numeric suffix among `req_ids` (0 when the domain is empty).
    pub max_req_num: u32,
}

impl From<&BaselineIndex> for Vec<DomainDetail> {
    fn from(index: &BaselineIndex) -> Self {
        let mut details: Self = index
            .domains()
            .map(|(domain, baseline)| {
                let mut req_ids: Vec<String> = baseline.ids.keys().cloned().collect();
                req_ids.sort();
                DomainDetail {
                    domain: domain.to_string(),
                    req_ids,
                    max_req_num: baseline.max_req_num,
                }
            })
            .collect();
        details.sort_by(|left, right| left.domain.cmp(&right.domain));
        details
    }
}

/// One bound source's contribution to the synthesis inputs.
///
/// Carries the source's inline `lead` and its `claims` passed through
/// verbatim from the parsed `evidence/<source>.yaml` so no body field
/// is lost — the agent reconciles over the full claim bodies. The
/// document-level `authority` is intentionally not carried.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct SourceInput {
    /// Plan source binding key matching `plan.yaml.sources.<key>`.
    pub source: String,
    /// The source's discovery lead id (from `evidence/<source>.yaml`).
    pub lead: String,
    /// The source's claims, passed through verbatim from the parsed
    /// Evidence document so every per-kind body field survives.
    pub claims: Vec<JsonValue>,
}

impl SourceInput {
    /// Shape one already-read Evidence document into a
    /// [`SourceInput`], pulling its `lead` and `claims` and
    /// dropping everything else (notably the document-level
    /// `authority`, which the kernel resolves post-response).
    ///
    /// # Errors
    ///
    /// Returns [`Error::YamlDe`] when `raw` is not valid YAML.
    pub(crate) fn from_yaml(source: &str, raw: &str) -> Result<Self> {
        let doc: JsonValue = serde_saphyr::from_str(raw)?;
        let lead = doc.get("lead").and_then(JsonValue::as_str).unwrap_or_default().to_string();
        let claims = doc.get("claims").and_then(JsonValue::as_array).cloned().unwrap_or_default();
        Ok(Self {
            source: source.to_string(),
            lead,
            claims,
        })
    }

    /// Read and shape one `evidence/<source>.yaml` into a
    /// [`SourceInput`].
    ///
    /// # Errors
    ///
    /// - [`error::Error::Filesystem`] when `path` cannot be read.
    /// - [`error::Error::YamlDe`] when the file is not valid YAML.
    pub fn from_file(source: &str, path: &Path) -> Result<Self> {
        Self::from_yaml(source, &project::fs::read_text(path)?)
    }
}

/// Assemble the agent synthesis step's input envelope from
/// already-read inputs.
///
/// `sources` is one [`SourceInput`] per bound source — the
/// caller builds the vec by reading each `evidence/<source>.yaml`
/// (e.g. via [`SourceInput::from_file`]).
/// `guidance_brief` is the bound target's resolved guidance body,
/// provided by the refine orchestration (which resolves the
/// [`project::adapter::TargetAdapter`] and reads the brief) so this
/// function stays pure and adapter-free.
#[must_use]
pub fn inputs(
    slice: &str, sources: &[SourceInput], guidance_brief: &str, baseline: &[Surface],
    baseline_detail: &[DomainDetail], baseline_decisions: &[Decision],
) -> SynthesisInputs {
    SynthesisInputs {
        version: SYNTHESIS_VERSION,
        kind: InputKind::Inputs,
        slice: slice.to_string(),
        sources: sources.to_vec(),
        guidance_brief: guidance_brief.to_string(),
        baseline: baseline.to_vec(),
        baseline_detail: baseline_detail.to_vec(),
        baseline_decisions: baseline_decisions.to_vec(),
    }
}
