//! Slice model — the typed `model.yaml` artifact.
//!
//! Kernel-owned fields are optional so the kernel re-stamps them on
//! projection (normalize, never reject); provenance stays inline.

use std::collections::BTreeMap;
use std::path::Path;

use artifacts::evidence::{AuthorityClass, Claim, ClaimKind};
use artifacts::spec::provenance::RequirementStatus;
use error::{Error, Result};
use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::provenance::{
    ContributingClaim, ProvenanceIndex, ProvenanceRequirement, ProvenanceResolution,
    ResolutionTrace,
};
use crate::synthesis::authority::{Agreement, ClaimRef, resolve};
use crate::synthesis::evidence::evidence_yaml_paths;

/// Domain used when a requirement has no explicit owner. Shared by
/// projection, spec rendering, and wave-commit identity so a
/// domain-omitted requirement resolves the same baseline everywhere.
pub const DEFAULT_DOMAIN: &str = "default";

/// In-memory view of `model.yaml`, holding the header, the requirement
/// set with inline provenance, and the task list.
///
/// The model carries only the earned core today — `requirements[]` and
/// `tasks[]`; the deferred non-requirements sections (`domain`, `apis`,
/// …) are not modeled yet and are ignored on deserialise. `target` is
/// persisted as the slice's `plan.yaml.targets` key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct SliceModel {
    /// Stored schema version. Kernel-stamped on the persisted file;
    /// optional because the agent response omits it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<u32>,
    /// Slice name. Kernel-stamped on the persisted file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slice: Option<String>,
    /// Bound `plan.yaml.targets` key. Kernel-stamped from the plan entry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
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
/// `scenarios`, `notes`, `domain`), the `agreement` verdict, optional
/// `baseline-id`, and the contributing `claims`; the kernel-owned
/// fields (`id`, `baseline-digest`, `status`, `sources`, claim `winner`)
/// are optional because the agent omits them and the kernel re-derives
/// them on projection. The `resolution` label is not stored here — the
/// provenance projection recomputes it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
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
    /// Agent-authored baseline `REQ` id when modifying an existing
    /// requirement in a domain that already has a merged baseline.
    /// The kernel keeps this reference and mints a slice-local `id`;
    /// omitted for additive requirements. Wave commit assigns the final
    /// baseline number later (RFC-86 D5).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub baseline_id: Option<String>,
    /// Kernel-stamped `sha256:…` digest of the baseline requirement body
    /// named by [`Self::baseline_id`]. Present only on `MODIFIED` rows;
    /// wave commit rejects drift when the baseline body no longer matches.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub baseline_digest: Option<String>,
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

impl ModelRequirement {
    /// Owning domain, falling back to [`DEFAULT_DOMAIN`] when omitted.
    #[must_use]
    pub fn domain_or_default(&self) -> &str {
        self.domain.as_deref().unwrap_or(DEFAULT_DOMAIN)
    }

    /// Canonical `sha256:<hex>` digest of this requirement's body
    /// content — the deferral match key (RFC-86a D2). Delegates to
    /// [`project::slice::RequirementBody`]: only the agent-authored
    /// body (title, statement, scenarios, notes) participates, so a
    /// re-refine that renumbers `REQ-NNN` ids keeps the digest while
    /// any body edit changes it.
    #[must_use]
    pub fn body_digest(&self) -> String {
        project::slice::RequirementBody {
            title: &self.title,
            statement: &self.statement,
            scenarios: &self.scenarios,
            notes: self.notes.as_deref(),
        }
        .digest()
    }
}

/// One inline claim under [`ModelRequirement::claims`].
///
/// The stable `(source, id, kind)` triple traces the claim to its
/// Evidence (the claim contract). The single-line `value` and
/// `path` anchor are read from `evidence/<source>.yaml` by the
/// provenance projection rather than copied here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
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

impl SliceModel {
    /// Parse a `model.yaml` from its raw contents. Unknown sections
    /// (`domain`, `apis`, …) are ignored on deserialise.
    ///
    /// # Errors
    ///
    /// [`Error::YamlDe`] when the contents are not valid YAML or do
    /// not fit the typed view.
    pub fn parse_yaml(raw: &str) -> Result<Self> {
        let model: Self = serde_saphyr::from_str(raw)?;
        Ok(model)
    }

    /// Load a `model.yaml` at `path`.
    ///
    /// # Errors
    ///
    /// - [`Error::Filesystem`] when `path` cannot be read.
    /// - [`Error::YamlDe`] when the file is not valid YAML or does not
    ///   fit the typed view.
    pub fn load(path: &Path) -> Result<Self> {
        Self::parse_yaml(&project::fs::read_text(path)?)
    }

    /// Project the audit-only provenance view from this model plus the
    /// slice's on-disk Evidence.
    ///
    /// Two derived fields are recomputed rather than read from the
    /// model: `resolution` (by re-running the authority kernel over
    /// the claims, per-source `authority`, overrides, and the persisted
    /// `agreement`) and each claim's `value` / `path` (read from
    /// `evidence/<source>.yaml`). `generated_at` / `generator` stamp
    /// the projection header.
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
                resolve(&claim_refs, &evidence.authority, overrides, req.agreement).label;
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
    /// and per-claim `value` / `path` are pulled from the typed
    /// document.
    ///
    /// # Errors
    ///
    /// - [`Error::Filesystem`] when an Evidence file cannot be read.
    /// - [`Error::YamlDe`] when an Evidence file is not valid YAML.
    fn read(slice_dir: &Path) -> Result<Self> {
        let mut index = Self::default();
        for path in evidence_yaml_paths(slice_dir)? {
            let raw = project::fs::read_text(&path)?;
            let document: artifacts::evidence::Document = serde_saphyr::from_str(&raw)?;
            let source = path.file_stem().and_then(|s| s.to_str()).unwrap_or_default().to_string();
            index.authority.insert(source.clone(), document.authority);
            for claim in document.claims {
                if let Some(id) = claim.id.clone() {
                    index.claims.insert((source.clone(), id), claim_body(&claim));
                }
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

/// Extract one claim's `value` and `path` from the typed claim.
///
/// `value` prefers the well-known open body fields in [`VALUE_FIELDS`]
/// order, then the first scalar string among the claim's open extras
/// (deterministic — the map iterates in key order), then the typed
/// `synopsis` / inline `payload`. `path` is the claim's source anchor,
/// read verbatim.
fn claim_body(claim: &Claim) -> ClaimBody {
    let value = VALUE_FIELDS
        .iter()
        .find_map(|field| claim.extras.get(*field).and_then(JsonValue::as_str))
        .or_else(|| claim.extras.values().find_map(JsonValue::as_str))
        .or(claim.synopsis.as_deref())
        .or(claim.payload.as_deref())
        .map(str::to_string);
    ClaimBody {
        value,
        path: claim.path.clone(),
    }
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
