//! Synthesis response wire DTO + input-envelope assembly.
//!
//! Synthesis is always agent-dispatched: there is no tool consumer, so
//! there is no closed *request* wire shape. The single schema-validated
//! wire is the **response**
//! ([`SynthesisResponse`], `kind: response`), validated against
//! `schemas/slice/synthesis.schema.json` by
//! `crate::schema_gate::validate_synthesis_json` before the refine
//! orchestration deserialises it here. The response carries the
//! agent's [`crate::slice::model::SliceModel`]
//! (kernel-owned and header fields omitted) plus the prose-only Markdown
//! [`SynthesisArtifacts`].
//!
//! The synthesis **inputs** the CLI hands the agent step are not
//! schema-validated (no closed request shape).
//! [`inputs`] assembles them — each bound
//! source's inline `lead` and `claims` plus the resolved target shape
//! brief body — into the plain serialisable [`SynthesisInputs`] the
//! guest refine orchestration hands the synthesis judgment. Authority
//! is **not** included: the kernel resolves it from the on-disk Evidence
//! after the response returns.
//!
//! The assembly is pure over already-read inputs so it unit-tests
//! without a temp project; [`SourceInput::from_file`]
//! is the only filesystem hook, kept off the core path and free of
//! adapter resolution (the refine orchestration resolves the
//! [`crate::adapter::TargetAdapter`] and reads the shape brief).

use std::path::Path;

use error::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::registry::topology::Surface;
use crate::slice::model::SliceModel;
use crate::slice::synthesis::baseline::BaselineIndex;

/// Wire version pinned by `schemas/slice/synthesis.schema.json`
/// (`version` `const: 1`) and echoed onto the input envelope.
const SYNTHESIS_VERSION: u32 = 1;

/// Synthesis response envelope kind.
///
/// Serialises to the literal `"response"` the schema's `const`
/// constraint requires. Mirrors `change::plan::core::propose`'s
/// `ProposalKind`, but synthesis has only the response kind — there is
/// no request wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SynthesisKind {
    /// The agent's synthesis result.
    Response,
}

/// `kind: response` envelope — the agent's synthesis result.
///
/// Round-trips `schemas/slice/synthesis.schema.json`. The DTO is
/// shape-only; the refine orchestration schema-gates the raw bytes via
/// `crate::schema_gate::validate_synthesis_json` before deserialising here,
/// and the projection kernel re-derives every kernel-owned field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
}

/// One per-domain spec body under [`SynthesisArtifacts::specs`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
}

/// Advisory per-domain baseline id facts for the synthesis inputs envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
        Self::from_yaml(source, &crate::fs::read_text(path)?)
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
/// [`crate::adapter::TargetAdapter`] and reads the brief) so this
/// function stays pure and adapter-free.
#[must_use]
pub fn inputs(
    slice: &str, sources: &[SourceInput], guidance_brief: &str, baseline: &[Surface],
    baseline_detail: &[DomainDetail],
) -> SynthesisInputs {
    SynthesisInputs {
        version: SYNTHESIS_VERSION,
        kind: InputKind::Inputs,
        slice: slice.to_string(),
        sources: sources.to_vec(),
        guidance_brief: guidance_brief.to_string(),
        baseline: baseline.to_vec(),
        baseline_detail: baseline_detail.to_vec(),
    }
}
