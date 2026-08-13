//! `system.yaml` — declared `identities[]` plus named architecture
//! states (RFC-104 D4): generated `as-is`, operator-owned `target`
//! and `transition-*`. Each named state digests independently.

pub mod overlay;

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use diagnostics::digest::sha256_hex;
use error::Error;
use project::snapshot::SnapshotId;
use serde::{Deserialize, Serialize};

/// The one supported `system.yaml` schema version.
const VERSION: u32 = 1;

/// The system model at `<system>/system.yaml`.
///
/// Unknown top-level keys are rejected by the `transition-*` grammar
/// check in [`Model::validate`] (serde's `deny_unknown_fields` cannot
/// combine with the flattened transition map).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Model {
    /// Schema version; only `1` is accepted.
    pub version: u32,
    /// Operator-owned stable identities; may be empty.
    #[serde(default)]
    pub identities: Vec<Identity>,
    /// The recovered model `system survey` writes.
    pub as_is: State,
    /// The intended end state (`system plan` writes it once when
    /// absent; operator-owned afterwards).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<State>,
    /// Intermediate `transition-*` states, keyed by their full name.
    #[serde(default, flatten)]
    pub transitions: BTreeMap<String, State>,
}

/// One operator-declared stable identity.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Identity {
    /// The canonical element or relationship id.
    pub id: String,
    /// Other names the same thing appears under.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    /// Ids this identity replaced.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub supersedes: Vec<String>,
}

/// One named architecture state.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct State {
    /// Stable elements, in model order.
    #[serde(default)]
    pub elements: Vec<Element>,
    /// Relationships between element ids.
    #[serde(default)]
    pub relationships: Vec<Relationship>,
}

/// One model element.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Element {
    /// Stable element id, unique across the state.
    pub id: String,
    /// Closed element vocabulary.
    pub kind: ElementKind,
    /// Epistemic status of this record.
    pub status: Status,
    /// Claim refs into persisted Evidence.
    #[serde(default)]
    pub claims: Vec<ClaimRef>,
    /// The `decisions/<id>.yaml` record behind `status: decided`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision: Option<String>,
    /// Relevant to the migration without being modified by it.
    #[serde(default)]
    pub context_only: bool,
    /// Open attribute map (D6 state and temporal facts live here).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub attributes: BTreeMap<String, String>,
}

/// One model relationship.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Relationship {
    /// Stable relationship id, unique across the state.
    pub id: String,
    /// Closed relationship vocabulary.
    pub kind: RelationshipKind,
    /// Source element id.
    pub from: String,
    /// Destination element id.
    pub to: String,
    /// Epistemic status of this record.
    pub status: Status,
    /// Claim refs into persisted Evidence.
    #[serde(default)]
    pub claims: Vec<ClaimRef>,
    /// The `decisions/<id>.yaml` record behind `status: decided`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision: Option<String>,
    /// Relevant to the migration without being modified by it.
    #[serde(default)]
    pub context_only: bool,
    /// Open attribute map.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub attributes: BTreeMap<String, String>,
}

/// One claim reference into persisted Evidence.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ClaimRef {
    /// Coverage-row source key.
    pub source: String,
    /// Claim id within that source's Evidence.
    pub id: String,
}

/// Closed element vocabulary (RFC-104 D4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ElementKind {
    /// A whole system within or beside the boundary.
    System,
    /// A service or component.
    Service,
    /// A source repository.
    Repository,
    /// An exposed interface (API, UI, file feed).
    Interface,
    /// A data store.
    DataStore,
    /// A queue or topic.
    Queue,
    /// A scheduled job.
    ScheduledJob,
    /// A deployment unit.
    DeploymentUnit,
    /// A runtime environment.
    Environment,
    /// An external actor or system.
    ExternalActor,
    /// An owning group.
    OwningGroup,
}

/// Closed relationship vocabulary (RFC-104 D4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum RelationshipKind {
    /// `from` contains `to`.
    Containment,
    /// `from` is deployed onto `to`.
    Deployment,
    /// `from` invokes `to`.
    Invocation,
    /// `from` publishes to `to`.
    Publication,
    /// `from` consumes from `to`.
    Consumption,
    /// `from` reads `to`.
    Read,
    /// `from` writes `to`.
    Write,
    /// `from` depends on `to`.
    Dependency,
    /// `from` owns `to`.
    Ownership,
}

/// Closed epistemic status set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    /// Backed by at least one persisted claim.
    Evidenced,
    /// Model inference with no claims; repetition never promotes it.
    Inferred,
    /// Disagreeing claims are retained, none wins.
    Conflict,
    /// An explicit gap with no claims.
    Unknown,
    /// Settled by a `decisions/<id>.yaml` record (persist-tail stamp;
    /// correlation cannot emit it).
    Decided,
}

impl Model {
    /// The empty first-creation model: no identities, empty `as-is`.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            version: VERSION,
            identities: Vec::new(),
            as_is: State::default(),
            target: None,
            transitions: BTreeMap::new(),
        }
    }

    /// Load and validate `system.yaml` from `path`.
    ///
    /// # Errors
    ///
    /// - `system-model-missing` when the file is absent.
    /// - `Error::YamlDe` for malformed YAML or unknown fields.
    /// - `system-model-invalid` for structural violations.
    pub fn load(path: &Path) -> Result<Self, Error> {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Err(Error::Diag {
                    code: "system-model-missing",
                    detail: format!("system.yaml not found at {}", path.display()),
                });
            }
            Err(err) => return Err(Error::Io(err)),
        };
        let model: Self = serde_saphyr::from_str(&text)?;
        model.validate()?;
        Ok(model)
    }

    /// Content digest of the whole file's canonical YAML encoding
    /// (the D10 `system-model` covered digest).
    ///
    /// # Errors
    ///
    /// Propagates YAML serialization failures.
    pub fn digest(&self) -> Result<SnapshotId, Error> {
        let yaml = artifacts::atomic::serialise_yaml(self)?;
        Ok(SnapshotId::from_digest(&sha256_hex(yaml.as_bytes())))
    }

    /// Look up a named state: `as-is`, `target`, or a `transition-*`.
    #[must_use]
    pub fn state(&self, name: &str) -> Option<&State> {
        match name {
            "as-is" => Some(&self.as_is),
            "target" => self.target.as_ref(),
            other => self.transitions.get(other),
        }
    }

    /// Validate the whole model.
    ///
    /// # Errors
    ///
    /// `system-model-invalid` (exit code 2) naming the violated rule.
    pub fn validate(&self) -> Result<(), Error> {
        let invalid = |rule: &str, detail: String| {
            Err(Error::validation_failed("system-model-invalid", rule, detail))
        };
        if self.version != VERSION {
            return invalid(
                "unsupported version",
                format!("system.yaml version {} is not {VERSION}", self.version),
            );
        }
        let mut names = BTreeSet::new();
        for id in self.identities.iter().map(|identity| identity.id.as_str()) {
            if id.trim().is_empty() {
                return invalid("identity id required", "an identity has an empty `id`".into());
            }
            if !names.insert(id) {
                return invalid(
                    "duplicate identity",
                    format!("identity `{id}` is declared more than once"),
                );
            }
        }
        let mut mapped = BTreeSet::new();
        for identity in &self.identities {
            for name in identity.aliases.iter().chain(&identity.supersedes) {
                if !mapped.insert(name.as_str()) || names.contains(name.as_str()) {
                    return invalid(
                        "ambiguous identity name",
                        format!("`{name}` maps to more than one identity"),
                    );
                }
            }
        }
        for key in self.transitions.keys() {
            let tail = key.strip_prefix("transition-").unwrap_or_default();
            if tail.is_empty() || !artifacts::evidence::is_kebab(tail) {
                return invalid(
                    "unknown field",
                    format!("`{key}` is not `identities`, a named state, or `transition-*`"),
                );
            }
        }
        self.as_is.validate("as-is")?;
        if let Some(target) = &self.target {
            target.validate("target")?;
        }
        for (name, state) in &self.transitions {
            state.validate(name)?;
        }
        Ok(())
    }
}

impl State {
    /// Content digest of this state's canonical YAML encoding —
    /// the identity every projection of the state names.
    ///
    /// # Errors
    ///
    /// Propagates YAML serialization failures.
    pub fn digest(&self) -> Result<SnapshotId, Error> {
        let yaml = artifacts::atomic::serialise_yaml(self)?;
        Ok(SnapshotId::from_digest(&sha256_hex(yaml.as_bytes())))
    }

    /// True when `id` names an element or relationship in this state.
    #[must_use]
    pub fn contains(&self, id: &str) -> bool {
        self.elements.iter().any(|element| element.id == id)
            || self.relationships.iter().any(|relationship| relationship.id == id)
    }

    pub(crate) fn validate(&self, name: &str) -> Result<(), Error> {
        let invalid = |rule: &str, detail: String| {
            Err(Error::validation_failed(
                "system-model-invalid",
                rule,
                format!("state `{name}`: {detail}"),
            ))
        };
        let mut ids = BTreeSet::new();
        for element in &self.elements {
            if element.id.trim().is_empty() {
                return invalid("id required", "an element has an empty `id`".into());
            }
            if !ids.insert(element.id.as_str()) {
                return invalid("duplicate id", format!("id `{}` appears twice", element.id));
            }
            coherent(&element.id, element.status, &element.claims, element.decision.as_deref())
                .map_or(Ok(()), |detail| invalid("status coherence", detail))?;
        }
        let elements: BTreeSet<&str> =
            self.elements.iter().map(|element| element.id.as_str()).collect();
        for relationship in &self.relationships {
            if relationship.id.trim().is_empty() {
                return invalid("id required", "a relationship has an empty `id`".into());
            }
            if !ids.insert(relationship.id.as_str()) {
                return invalid("duplicate id", format!("id `{}` appears twice", relationship.id));
            }
            for end in [&relationship.from, &relationship.to] {
                if !elements.contains(end.as_str()) {
                    return invalid(
                        "unresolved endpoint",
                        format!(
                            "relationship `{}` endpoint `{end}` is not an element id",
                            relationship.id
                        ),
                    );
                }
            }
            coherent(
                &relationship.id,
                relationship.status,
                &relationship.claims,
                relationship.decision.as_deref(),
            )
            .map_or(Ok(()), |detail| invalid("status coherence", detail))?;
        }
        Ok(())
    }
}

/// Status/claims/decision coherence for one record; `Some(detail)` on
/// violation.
fn coherent(
    id: &str, status: Status, claims: &[ClaimRef], decision: Option<&str>,
) -> Option<String> {
    if decision.is_some() != (status == Status::Decided) {
        return Some(format!("`{id}`: `decision` is present iff `status` is `decided`"));
    }
    match status {
        Status::Evidenced | Status::Conflict if claims.is_empty() => {
            Some(format!("`{id}`: this `status` requires at least one claim"))
        }
        Status::Inferred | Status::Unknown if !claims.is_empty() => {
            Some(format!("`{id}`: this `status` must carry no claims"))
        }
        _ => None,
    }
}
