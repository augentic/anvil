//! Slice model — `model.yaml`.
//!
//! One structured artifact per slice at
//! `.specify/slices/<slice>/model.yaml`. The single
//! `schemas/slice/model.schema.json` validates both the agent's
//! synthesis-response `model` and the persisted file: kernel-owned and
//! header fields are optional so the kernel re-derives/stamps them on
//! projection (normalize, never reject). Provenance is carried inline
//! on each requirement, so the provenance view is *projected* on
//! demand by `specify slice provenance` rather than persisted as a
//! second file. See [`DECISIONS.md` §"Single slice-model artifact"][model-artifact].
//!
//! [model-artifact]: ../../../../DECISIONS.md#single-slice-model-artifact

use std::collections::BTreeMap;
use std::path::Path;

use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use specify_error::{Error, Result};
use specify_model::evidence::{AuthorityClass, ClaimKind};
use specify_model::spec::provenance::RequirementStatus;
use specify_schema::{ValidationStatus, join_details};

use crate::schema::{SLICE_MODEL_JSON_SCHEMA, evidence_yaml_paths, validate_value};
use crate::slice::provenance::{
    ContributingClaim, ProvenanceIndex, ProvenanceRequirement, ProvenanceResolution,
    ResolutionTrace,
};
use crate::slice::synthesis::authority::{Agreement, ClaimRef, resolve};

/// In-memory view of `model.yaml`, holding the header, the requirement
/// set with inline provenance, and the task list.
///
/// The model carries only the earned core today — `requirements[]` and
/// `tasks[]`; the deferred non-requirements sections (`domain`, `apis`,
/// …) are not part of the schema yet. The top-level shape is closed
/// (`additionalProperties: false`), enforced by the embedded schema
/// during [`SliceModel::load`]. `target` is not persisted — it is
/// resolved on demand from the bound `project`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SliceModel {
    /// Stored schema version. Kernel-stamped on the persisted file;
    /// optional because the agent response omits it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<u32>,
    /// Slice name. Kernel-stamped on the persisted file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slice: Option<String>,
    /// Bound project, optional.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    /// The requirement set with inline provenance.
    #[serde(default)]
    pub requirements: Vec<ModelRequirement>,
    /// Requirement→task tracing list.
    #[serde(default)]
    pub tasks: Vec<ModelTask>,
}

/// One `requirements[]` entry.
///
/// The agent authors the behavioral prose (`title`, `statement`,
/// `scenarios`, `notes`, `domain`), the `agreement` verdict, and the
/// contributing `claims`; the kernel-owned fields (`id`, `status`,
/// `sources`, claim `winner`) are optional because the agent omits them
/// and the kernel re-derives them on projection. The `resolution` label
/// is not stored here — the provenance projection recomputes it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ModelRequirement {
    /// Kernel-projected `REQ-NNN` id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Agent-authored requirement title.
    pub title: String,
    /// Kernel-projected status.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<RequirementStatus>,
    /// Agent-authored agreement verdict over the contributing claims.
    /// Present only when more than one claim contributes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agreement: Option<Agreement>,
    /// Agent-authored owning domain (kebab-case spec group).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    /// Kernel-projected rendered source list (highest authority first).
    #[serde(default)]
    pub sources: Vec<String>,
    /// Agent-authored contributing claims with kernel-projected
    /// `winner` markers. The claim `value` / `path` payload is read
    /// from on-disk Evidence by the provenance projection, not persisted
    /// here.
    #[serde(default)]
    pub claims: Vec<ModelClaim>,
    /// Agent-authored behavioral statement.
    pub statement: String,
    /// Agent-authored scenario lines.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scenarios: Vec<String>,
    /// Agent-authored free-form notes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// One inline claim under [`ModelRequirement::claims`].
///
/// The stable `(source, id, kind)` triple traces the claim to its
/// Evidence (the claim contract). The single-line `value` and
/// `path` anchor are read from `evidence/<source>.yaml` by the
/// provenance projection rather than copied here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ModelClaim {
    /// Source key the claim came from.
    pub source: String,
    /// Claim id within that source's Evidence file.
    pub id: String,
    /// Claim kind.
    pub kind: ClaimKind,
    /// Kernel-projected winner marker (divergence only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub winner: Option<bool>,
}

/// One `tasks[]` entry. Ids follow the
/// `TASK-NNN` / `REQ-NNN` grammars; grammar validation lives in the
/// drift validators, so these are plain strings here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ModelTask {
    /// Agent-authored `TASK-NNN` id.
    pub id: String,
    /// Agent-authored task text.
    pub text: String,
    /// `TASK-NNN` ids that must complete before this task.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<String>,
    /// `REQ-NNN` ids this task satisfies.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub satisfies: Vec<String>,
}

/// Validate a raw `model.yaml` document (parsed to JSON) against the
/// embedded `schemas/slice/model.schema.json`.
///
/// Validates the *whole* document — including the non-requirements
/// sections [`SliceModel`] does not model — so the closed top-level
/// shape is enforced even though the typed view captures only the
/// header and requirements.
///
/// # Errors
///
/// Returns [`Error::Validation`] keyed on `"slice-model-schema"` when
/// the document fails the schema.
pub fn validate_model_doc(value: &JsonValue) -> Result<()> {
    let rule = "model.yaml conforms to schemas/slice/model.schema.json";
    let failures: Vec<_> =
        validate_value(value, SLICE_MODEL_JSON_SCHEMA, "slice-model-schema", rule)
            .into_iter()
            .filter(|summary| summary.status == ValidationStatus::Fail)
            .collect();
    if failures.is_empty() {
        Ok(())
    } else {
        Err(Error::Validation {
            code: "slice-model-schema".into(),
            detail: join_details(&failures),
        })
    }
}

impl SliceModel {
    /// Parse and schema-validate a `model.yaml` from its raw contents.
    ///
    /// The whole document is validated against the schema first, then
    /// the typed header + requirements view is deserialised from it.
    ///
    /// # Errors
    ///
    /// - [`Error::YamlDe`] when the contents are not valid YAML.
    /// - [`Error::Validation`] when the document fails the schema.
    pub fn parse_yaml(raw: &str) -> Result<Self> {
        let value: JsonValue = serde_saphyr::from_str(raw)?;
        validate_model_doc(&value)?;
        // Re-parse into the typed header + requirements view; unknown
        // sections (`domain`, `apis`, …) are ignored on deserialise,
        // having already been schema-checked on the raw document above.
        let model: Self = serde_saphyr::from_str(raw)?;
        Ok(model)
    }

    /// Load and schema-validate a `model.yaml` at `path`.
    ///
    /// # Errors
    ///
    /// - [`Error::Filesystem`] when `path` cannot be read.
    /// - [`Error::YamlDe`] when the file is not valid YAML.
    /// - [`Error::Validation`] when the file fails the schema.
    pub fn load(path: &Path) -> Result<Self> {
        let raw = std::fs::read_to_string(path).map_err(|source| Error::Filesystem {
            op: "read",
            path: path.to_path_buf(),
            source,
        })?;
        Self::parse_yaml(&raw)
    }

    /// Project the audit-only provenance view from this model plus the
    /// slice's on-disk Evidence.
    ///
    /// The persisted model carries the load-bearing provenance inline
    /// (`status`, claim `winner` markers, rendered `sources`); the two
    /// derived fields are **recomputed** rather than read from the
    /// model:
    ///
    /// - `resolution` (and `resolution-trace`) is re-derived by re-running
    ///   the authority kernel ([`resolve`]) over the requirement's
    ///   `ClaimRef`s, the per-source document `authority` read from
    ///   `evidence/<source>.yaml`, the per-slice `overrides` map, and the
    ///   requirement's persisted `agreement` verdict.
    /// - each contributing claim's `value` (single-line payload) and
    ///   `path` anchor are read from `evidence/<source>.yaml`, keyed by
    ///   the `(source, id)` the claim already carries.
    ///
    /// `generated_at` and `generator` stamp the projection's header so
    /// it round-trips against `schemas/slice/provenance.schema.json`.
    ///
    /// # Errors
    ///
    /// - [`Error::Validation`] keyed on `"slice-model-incomplete"` when a
    ///   persisted requirement is missing the kernel-owned `id` /
    ///   `status` fields the projection requires (i.e. the model was a
    ///   pre-projection agent draft, not a persisted artifact).
    /// - [`Error::Filesystem`] / [`Error::YamlDe`] when an
    ///   `evidence/*.yaml` cannot be read or parsed.
    pub fn to_provenance_index(
        &self, slice_dir: &Path, overrides: &BTreeMap<ClaimKind, String>, generated_at: Timestamp,
        generator: String,
    ) -> Result<ProvenanceIndex> {
        let evidence = EvidenceIndex::read(slice_dir)?;
        let mut requirements = Vec::with_capacity(self.requirements.len());
        for req in &self.requirements {
            let id = req.id.clone().ok_or_else(|| missing_field("requirements[].id"))?;
            let status = req.status.ok_or_else(|| missing_field("requirements[].status"))?;
            let claim_refs: Vec<ClaimRef> = req
                .claims
                .iter()
                .map(|c| ClaimRef {
                    source: c.source.clone(),
                    id: c.id.clone(),
                    kind: c.kind,
                })
                .collect();
            let resolved =
                resolve(&claim_refs, &evidence.authority, overrides, req.agreement).resolution;
            let contributing_claims: Vec<ContributingClaim> = req
                .claims
                .iter()
                .map(|c| {
                    let body = evidence.claim(&c.source, &c.id);
                    ContributingClaim {
                        source: c.source.clone(),
                        id: c.id.clone(),
                        kind: c.kind,
                        value: body.and_then(|b| b.value.clone()),
                        path: body.and_then(|b| b.path.clone()),
                        winner: c.winner,
                    }
                })
                .collect();
            let resolution_trace = resolution_trace(resolved, req);
            requirements.push(ProvenanceRequirement {
                id,
                status,
                sources: req.sources.clone(),
                contributing_claims,
                resolution: resolved,
                resolution_trace,
            });
        }
        let index = ProvenanceIndex {
            version: 1,
            slice: self.slice.clone().unwrap_or_default(),
            generated_at,
            generator,
            requirements,
        };
        index.validate()?;
        Ok(index)
    }
}

/// Build the optional [`ResolutionTrace`] for a projected requirement.
///
/// A trace is emitted only for the two authority-decided resolutions
/// ([`ProvenanceResolution::AuthorityResolved`] /
/// [`ProvenanceResolution::PerSliceOverride`]); the agreement, single,
/// unknown, and tied-conflict cases have no tie to narrate. The winner
/// source is the claim the kernel marked `winner: true` inline.
fn resolution_trace(
    resolution: ProvenanceResolution, req: &ModelRequirement,
) -> Option<ResolutionTrace> {
    let step = match resolution {
        ProvenanceResolution::AuthorityResolved => "default-authority-ordering",
        ProvenanceResolution::PerSliceOverride => "per-slice-authority-override",
        _ => return None,
    };
    let winner = req.claims.iter().find(|c| c.winner == Some(true)).map(|c| c.source.clone());
    Some(ResolutionTrace {
        step: step.to_string(),
        r#override: None,
        winner,
    })
}

/// The single-line payload and `<path>#L<n>` anchor of one Evidence
/// claim, read from `evidence/<source>.yaml` for the provenance
/// projection.
#[derive(Debug, Default)]
struct ClaimBody {
    /// First-line claim payload (`statement` / `criterion` / … body).
    value: Option<String>,
    /// `<path>#L<n>` anchor.
    path: Option<String>,
}

/// Per-slice Evidence index keyed for provenance projection: the
/// document-level `authority` per source and the `(source, id)` →
/// [`ClaimBody`] lookup.
#[derive(Debug, Default)]
struct EvidenceIndex {
    /// Source key → document-level [`AuthorityClass`].
    authority: BTreeMap<String, AuthorityClass>,
    /// `(source, id)` → claim body payload.
    claims: BTreeMap<(String, String), ClaimBody>,
}

impl EvidenceIndex {
    /// Read every `evidence/*.yaml` under `slice_dir` into the index.
    /// Source key is each file stem; the document-level `authority`
    /// and per-claim `value` / `path` are pulled from the parsed JSON.
    ///
    /// # Errors
    ///
    /// - [`Error::Filesystem`] when an Evidence file cannot be read.
    /// - [`Error::YamlDe`] when an Evidence file is not valid YAML.
    fn read(slice_dir: &Path) -> Result<Self> {
        let mut index = Self::default();
        for path in evidence_yaml_paths(slice_dir)? {
            let raw = std::fs::read_to_string(&path).map_err(|source| Error::Filesystem {
                op: "read",
                path: path.clone(),
                source,
            })?;
            let doc: JsonValue = serde_saphyr::from_str(&raw)?;
            let source = path.file_stem().and_then(|s| s.to_str()).unwrap_or_default().to_string();
            if let Some(class) = doc
                .get("authority")
                .and_then(JsonValue::as_str)
                .and_then(|s| serde_json::from_value(JsonValue::String(s.to_string())).ok())
            {
                index.authority.insert(source.clone(), class);
            }
            let Some(claims) = doc.get("claims").and_then(JsonValue::as_array) else {
                continue;
            };
            for claim in claims {
                let Some(id) = claim.get("id").and_then(JsonValue::as_str) else {
                    continue;
                };
                index.claims.insert((source.clone(), id.to_string()), claim_body(claim));
            }
        }
        Ok(index)
    }

    /// Look up one claim body by `(source, id)`.
    fn claim(&self, source: &str, id: &str) -> Option<&ClaimBody> {
        self.claims.get(&(source.to_string(), id.to_string()))
    }
}

/// Closed list of preferred single-line `value` body fields, in
/// precedence order. A `requirement`
/// carries `statement`, a `criterion` carries `criterion`, a `decision`
/// carries `decision`, an `example` carries `output`.
const VALUE_FIELDS: [&str; 4] = ["statement", "criterion", "decision", "output"];

/// Extract one claim's `value` and `path` from its parsed JSON object.
///
/// `value` prefers the well-known body fields in [`VALUE_FIELDS`] order,
/// then falls back to the first scalar string body field that is not the
/// `id` / `kind` / `path` structural keys. `path` is read verbatim.
fn claim_body(claim: &JsonValue) -> ClaimBody {
    let value = VALUE_FIELDS
        .iter()
        .find_map(|field| claim.get(*field).and_then(JsonValue::as_str))
        .or_else(|| first_scalar_body(claim))
        .map(str::to_string);
    let path = claim.get("path").and_then(JsonValue::as_str).map(str::to_string);
    ClaimBody { value, path }
}

/// First scalar string body field of a claim object, skipping the
/// `id` / `kind` / `path` structural keys. Deterministic — the parsed
/// object iterates in key order.
fn first_scalar_body(claim: &JsonValue) -> Option<&str> {
    claim
        .as_object()?
        .iter()
        .filter(|(key, _)| !matches!(key.as_str(), "id" | "kind" | "path"))
        .find_map(|(_, v)| v.as_str())
}

fn missing_field(field: &str) -> Error {
    Error::validation_failed(
        "slice-model-incomplete",
        "a persisted model.yaml carries kernel-projected provenance fields",
        format!(
            "{field} is absent; the provenance projection requires a persisted (projected) \
             model.yaml, not a pre-projection synthesis draft"
        ),
    )
}

#[cfg(test)]
mod tests;
