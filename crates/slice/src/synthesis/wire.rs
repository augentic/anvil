//! Synthesis response wire DTO + input-envelope assembly.
//!
//! [`SynthesisResponse`] is the single schema-gated wire shape; the
//! inputs envelope has no closed request shape and assembly is pure.

use std::path::Path;

use error::Result;
use project::identity::{Decision, Surface};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::model::SliceModel;
use crate::synthesis::baseline::BaselineIndex;

/// Wire version stamped on both the input and response envelopes.
/// v2: `sources[]` entries carry a project-relative `evidence-path`
/// instead of inlined `claims[]` — the agent reads each Evidence
/// document from the lent tree.
const SYNTHESIS_VERSION: u32 = 2;

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
    /// One entry per bound source, carrying its `lead` and the
    /// project-relative `evidence-path` to its Evidence document.
    pub sources: Vec<SourceInput>,
    /// The resolved target guidance body. Resolved and read by the
    /// refine orchestration — never by this module.
    pub guidance_brief: String,
    /// The slice's bound project baseline surface (one entry
    /// per `.emery/specs/<domain>/spec.md`), so synthesis reconciles
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
    /// Ordered predecessor refinement context (RFC-91 D3): each
    /// predecessor's refinement digest and readable artifact root.
    /// Change-local context only — never Source Evidence, and it does
    /// not alter artifact authority. Empty stays off the wire.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<DependencyContext>,
}

/// One predecessor's refinement context under
/// [`SynthesisInputs::dependencies`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct DependencyContext {
    /// Predecessor slice name.
    pub slice: String,
    /// Predecessor refinement digest (`sha256:…`).
    pub refinement: String,
    /// Project-relative readable artifact root
    /// (`.emery/slices/<predecessor>/`).
    pub artifacts_root: String,
}

/// Advisory per-domain baseline id facts for the synthesis inputs envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct DomainDetail {
    /// Domain directory slug under `.emery/specs/`.
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
/// Carries the source's `lead` and the project-relative
/// `evidence-path` to its Evidence document in the lent tree — the
/// agent reads the claim bodies from that file instead of receiving
/// them inline on the wire. The document-level `authority` is
/// intentionally not carried.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct SourceInput {
    /// Plan source binding key matching `plan.yaml.sources.<key>`.
    pub source: String,
    /// The source's discovery lead id (from `evidence/<source>.yaml`).
    pub lead: String,
    /// Project-relative path to the source's Evidence document
    /// (`.emery/slices/<slice>/evidence/<source>.yaml`), resolvable in
    /// the lent tree the agent works in.
    pub evidence_path: String,
}

impl SourceInput {
    /// Shape one already-read Evidence document into a
    /// [`SourceInput`], pulling its `lead` and recording
    /// `evidence_path` as the wire path — the claims stay on disk for
    /// the agent to read (and the document-level `authority` is
    /// resolved by the kernel post-response).
    ///
    /// # Errors
    ///
    /// Returns [`Error::YamlDe`] when `raw` is not valid YAML.
    pub(crate) fn from_yaml(source: &str, raw: &str, evidence_path: String) -> Result<Self> {
        let doc: JsonValue = serde_saphyr::from_str(raw)?;
        let lead = doc.get("lead").and_then(JsonValue::as_str).unwrap_or_default().to_string();
        Ok(Self {
            source: source.to_string(),
            lead,
            evidence_path,
        })
    }

    /// Read one `evidence/<source>.yaml` at `path` into a
    /// [`SourceInput`] carrying `evidence_path` as its wire path.
    ///
    /// # Errors
    ///
    /// - [`error::Error::Filesystem`] when `path` cannot be read.
    /// - [`error::Error::YamlDe`] when the file is not valid YAML.
    pub fn from_file(source: &str, path: &Path, evidence_path: String) -> Result<Self> {
        Self::from_yaml(source, &project::fs::read_text(path)?, evidence_path)
    }
}

/// Assemble the agent synthesis step's input envelope from
/// already-read inputs.
///
/// `sources` is one [`SourceInput`] per bound source;
/// `guidance_brief` is the bound target's resolved guidance body, read
/// by the refine orchestration so this function stays adapter-free;
/// `dependencies` is the ordered predecessor refinement context.
#[must_use]
pub fn inputs(
    slice: &str, sources: &[SourceInput], guidance_brief: &str, baseline: &[Surface],
    baseline_detail: &[DomainDetail], baseline_decisions: &[Decision],
    dependencies: &[DependencyContext],
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
        dependencies: dependencies.to_vec(),
    }
}
