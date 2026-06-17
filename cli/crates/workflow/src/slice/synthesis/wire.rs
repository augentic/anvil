//! Synthesis response wire DTO + input-envelope assembly.
//!
//! Synthesis is always agent-dispatched: there is no tool consumer, so
//! there is no closed *request* wire shape. The single schema-validated
//! wire is the **response**
//! ([`SynthesisResponse`], `kind: response`), validated against
//! `schemas/slice/synthesis.schema.json` by
//! [`crate::schema::validate_synthesis_json`] before C8 deserialises it
//! here. The response carries the agent's [`crate::slice::model::SliceModel`]
//! (kernel-owned and header fields omitted) plus the prose-only Markdown
//! [`SynthesisArtifacts`].
//!
//! The synthesis **inputs** the CLI hands the agent step are not
//! schema-validated (no closed request shape).
//! [`build_synthesis_inputs`] assembles them — each bound
//! source's inline `lead` and `claims` plus the resolved target shape
//! brief body — into the plain serialisable [`SynthesisInputs`] that
//! `specify slice synthesize --dry-run --format json` prints. Authority
//! is **not** included: the kernel resolves it from the on-disk Evidence
//! after the response returns.
//!
//! The assembly is pure over already-read inputs so it unit-tests
//! without a temp project; [`SynthesisSourceInput::from_evidence_file`]
//! is the only filesystem hook, kept off the core path and free of
//! adapter resolution (C8 resolves the [`crate::adapter::TargetAdapter`]
//! and reads the shape brief).

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use specify_error::{Error, Result};

use crate::registry::topology::Surface;
use crate::slice::model::SliceModel;

/// Wire version pinned by `schemas/slice/synthesis.schema.json`
/// (`version` `const: 1`) and echoed onto the input envelope.
const SYNTHESIS_VERSION: u32 = 1;

/// Closed `kind` discriminator for the synthesis response.
///
/// Serialises to the literal `"response"` the schema's `const`
/// constraint requires. Mirrors `change::plan::core::propose`'s
/// `ProposalKind`, but synthesis has only the response kind — there is
/// no request wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SynthesisKind {
    /// `kind: response` — the agent's synthesis result the CLI reads
    /// back.
    Response,
}

/// `kind: response` envelope — the agent's synthesis result.
///
/// Round-trips `schemas/slice/synthesis.schema.json`. The DTO is
/// shape-only; C8 schema-gates the raw bytes via
/// [`crate::schema::validate_synthesis_json`] before deserialising here,
/// and the projection kernel re-derives every kernel-owned field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SynthesisResponse {
    /// Wire version; always `1` per the schema `const`.
    pub version: u32,
    /// Discriminator; always [`SynthesisKind::Response`].
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

/// Closed `kind` discriminator for the synthesis input envelope.
///
/// The inputs are not schema-validated (there is no closed request
/// shape), but the envelope still carries a
/// closed discriminator for symmetry with the response.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SynthesisInputsKind {
    /// `kind: inputs` — the agent synthesis step's input envelope.
    Inputs,
}

/// The agent synthesis step's input envelope.
///
/// Assembled by [`build_synthesis_inputs`] and printed by `specify
/// slice synthesize --dry-run --format json`. Not schema-validated —
/// synthesis is always agent-dispatched, so there is no tool consumer
/// and no closed request schema. Authority is deliberately absent: the
/// kernel resolves it post-response from on-disk Evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SynthesisInputs {
    /// Envelope version, mirroring the response.
    pub version: u32,
    /// Discriminator; always [`SynthesisInputsKind::Inputs`].
    pub kind: SynthesisInputsKind,
    /// Slice name the step synthesises.
    pub slice: String,
    /// One entry per bound source, carrying its inline `lead` and
    /// `claims`.
    pub sources: Vec<SynthesisSourceInput>,
    /// The resolved target `shape` brief body. Resolved and read by C8 —
    /// never by this module.
    pub shape_brief: String,
    /// RFC-46 D5 — the slice's bound project baseline surface (one entry
    /// per `.specify/specs/<domain>/spec.md`), so synthesis reconciles
    /// against existing requirements instead of duplicating them. Read
    /// from the project topology at assembly time; empty (greenfield, or
    /// no baseline) stays off the wire. Context only — no schema or
    /// `model.yaml` change.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub baseline: Vec<Surface>,
}

/// One bound source's contribution to the synthesis inputs.
///
/// Carries the source's inline `lead` and its `claims` passed through
/// verbatim from the parsed `evidence/<source>.yaml` so no body field
/// is lost — the agent reconciles over the full claim bodies. The
/// document-level `authority` is intentionally not carried.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SynthesisSourceInput {
    /// Plan source binding key matching `plan.yaml.sources.<key>`.
    pub source: String,
    /// The source's discovery lead id (from `evidence/<source>.yaml`).
    pub lead: String,
    /// The source's claims, passed through verbatim from the parsed
    /// Evidence document so every per-kind body field survives.
    pub claims: Vec<JsonValue>,
}

impl SynthesisSourceInput {
    /// Shape one already-read Evidence document into a
    /// [`SynthesisSourceInput`], pulling its `lead` and `claims` and
    /// dropping everything else (notably the document-level
    /// `authority`, which the kernel resolves post-response).
    ///
    /// # Errors
    ///
    /// Returns [`Error::YamlDe`] when `raw` is not valid YAML.
    pub(crate) fn from_evidence_yaml(source: &str, raw: &str) -> Result<Self> {
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
    /// [`SynthesisSourceInput`].
    ///
    /// # Errors
    ///
    /// - [`Error::Filesystem`] when `path` cannot be read.
    /// - [`Error::YamlDe`] when the file is not valid YAML.
    pub fn from_evidence_file(source: &str, path: &Path) -> Result<Self> {
        let raw = std::fs::read_to_string(path).map_err(|err| Error::Filesystem {
            op: "read",
            path: path.to_path_buf(),
            source: err,
        })?;
        Self::from_evidence_yaml(source, &raw)
    }
}

/// Assemble the agent synthesis step's input envelope from
/// already-read inputs.
///
/// `sources` is one [`SynthesisSourceInput`] per bound source — the
/// caller builds the vec by reading each `evidence/<source>.yaml`
/// (e.g. via [`SynthesisSourceInput::from_evidence_file`]).
/// `shape_brief` is the bound target's resolved `shape` brief body,
/// provided by C8 (which resolves the [`crate::adapter::TargetAdapter`]
/// and reads the brief) so this function stays pure and adapter-free.
#[must_use]
pub fn build_synthesis_inputs(
    slice: &str, sources: &[SynthesisSourceInput], shape_brief: &str, baseline: &[Surface],
) -> SynthesisInputs {
    SynthesisInputs {
        version: SYNTHESIS_VERSION,
        kind: SynthesisInputsKind::Inputs,
        slice: slice.to_string(),
        sources: sources.to_vec(),
        shape_brief: shape_brief.to_string(),
        baseline: baseline.to_vec(),
    }
}

#[cfg(test)]
mod tests;
