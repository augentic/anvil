//! `migration.yaml` — inlined modernization dispositions and
//! migration waves (RFC-104 D7/D9). Sub-records are not extra files;
//! the handoff references them as `{ id, digest }`.

use std::collections::BTreeSet;
use std::path::Path;

use diagnostics::digest::sha256_hex;
use error::Error;
use project::snapshot::SnapshotId;
use serde::{Deserialize, Serialize};

/// The one supported `migration.yaml` schema version.
const VERSION: u32 = 1;

/// The migration plan at `<system>/migration.yaml`.
///
/// Operator-owned once written: `system plan` writes it only as part
/// of the initial architecture proposal and never overwrites operator
/// edits afterwards.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Migration {
    /// Schema version; only `1` is accepted.
    pub version: u32,
    /// Reviewed modernization dispositions.
    #[serde(default)]
    pub dispositions: Vec<Disposition>,
    /// Bounded migration waves, in plan order.
    #[serde(default)]
    pub waves: Vec<Wave>,
}

/// One reviewed modernization disposition (RFC-104 D7).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Disposition {
    /// Stable disposition id, unique across the file.
    pub id: String,
    /// The reviewed treatment.
    pub treatment: Treatment,
    /// Element or relationship ids the disposition covers.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub applies_to: Vec<String>,
    /// The reviewed reason (desired outcome and authority for
    /// `change`; the insufficiency for `investigate`).
    pub reason: String,
}

/// Closed treatment set (RFC-104 D7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum Treatment {
    /// Required behaviour or constraint that must survive.
    Preserve,
    /// Intentional divergence.
    Change,
    /// Intentionally removed.
    Retire,
    /// Insufficient evidence or authority for a responsible decision.
    Investigate,
}

/// One bounded migration wave (RFC-104 D9).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Wave {
    /// Stable wave id, unique across the file.
    pub id: String,
    /// The bounded outcome.
    pub outcome: String,
    /// Named `system.yaml` states before and after the wave.
    pub architecture: WaveArchitecture,
    /// Predecessor wave ids within this plan.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub predecessors: Vec<String>,
    /// External preconditions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub preconditions: Vec<Item>,
    /// Elements that may experience an observable consequence.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub affected_elements: Vec<String>,
    /// Elements in the delivery ownership envelope.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub touched_elements: Vec<String>,
    /// Read-only architectural context.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub context_elements: Vec<String>,
    /// Disposition ids this wave enacts (refs into `dispositions[]`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dispositions: Vec<String>,
    /// The selected estate-survey leads delivery will import.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence_scopes: Vec<EvidenceScope>,
    /// Proposed delivery targets, including targets that must be
    /// created before RFC-88 authoring.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub targets: Vec<TargetRef>,
    /// Reviewed source-to-target assignments.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub delivery_mappings: Vec<DeliveryMapping>,
    /// State movement and reconciliation.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub state_movements: Vec<Item>,
    /// Coexistence requirements.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub coexistence: Vec<Item>,
    /// Cutover requirements.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cutover: Vec<Item>,
    /// Rollback position.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rollback: Vec<Item>,
    /// Operational-readiness requirements.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub operational_readiness: Vec<Item>,
    /// The acceptance boundary.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub acceptance: Vec<Item>,
    /// Verification expectations.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub verification: Vec<Item>,
    /// Conservation expectations (RFC-98 consumes them later).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conservation: Vec<Item>,
    /// Material unknowns.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gaps: Vec<Item>,
    /// Commercial assumptions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub assumptions: Vec<Item>,
    /// Authority decisions (refs to `decisions/<id>.yaml`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub decisions: Vec<String>,
}

/// The named states a wave moves between.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct WaveArchitecture {
    /// Named `system.yaml` state before the wave.
    pub before: String,
    /// Named `system.yaml` state after the wave.
    pub after: String,
}

/// One inlined wave sub-record: a stable id plus its reviewed detail.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Item {
    /// Stable sub-record id, unique within its list.
    pub id: String,
    /// The reviewed detail.
    pub detail: String,
}

/// One selected estate-survey lead.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct EvidenceScope {
    /// Coverage-row source key.
    pub source: String,
    /// The surveyed lead id.
    pub lead: String,
}

/// One proposed delivery target.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TargetRef {
    /// Reviewed logical target id.
    pub id: String,
    /// Mutable origin locator (URL or path).
    pub locator: String,
    /// Operator-declared adapter identity (a bare name or an exact
    /// package pin) — copied into the handoff exactly as declared.
    pub adapter: String,
}

/// One reviewed source-to-target assignment.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct DeliveryMapping {
    /// Coverage-row source key.
    pub source: String,
    /// The surveyed lead id.
    pub lead: String,
    /// A `targets[]` id in the same wave.
    pub target: String,
}

impl Migration {
    /// Load and validate `migration.yaml` from `path`.
    ///
    /// # Errors
    ///
    /// - `system-migration-missing` when the file is absent.
    /// - `Error::YamlDe` for malformed YAML or unknown fields.
    /// - `system-migration-invalid` for structural violations.
    pub fn load(path: &Path) -> Result<Self, Error> {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Err(Error::Diag {
                    code: "system-migration-missing",
                    detail: format!("migration.yaml not found at {}", path.display()),
                });
            }
            Err(err) => return Err(Error::Io(err)),
        };
        let migration: Self = serde_saphyr::from_str(&text)?;
        migration.validate()?;
        Ok(migration)
    }

    /// Content digest of the whole file's canonical YAML encoding
    /// (the D10 `migration-plan` covered digest).
    ///
    /// # Errors
    ///
    /// Propagates YAML serialization failures.
    pub fn digest(&self) -> Result<SnapshotId, Error> {
        let yaml = artifacts::atomic::serialise_yaml(self)?;
        Ok(SnapshotId::from_digest(&sha256_hex(yaml.as_bytes())))
    }

    /// Look up one disposition by id.
    #[must_use]
    pub fn disposition(&self, id: &str) -> Option<&Disposition> {
        self.dispositions.iter().find(|disposition| disposition.id == id)
    }

    /// Look up one wave by id.
    #[must_use]
    pub fn wave(&self, id: &str) -> Option<&Wave> {
        self.waves.iter().find(|wave| wave.id == id)
    }

    /// Validate the file's internal structure. Cross-file resolution
    /// (named states, decision records, model elements, coverage rows)
    /// is the handoff projection's fail-closed gate.
    ///
    /// # Errors
    ///
    /// `system-migration-invalid` (exit code 2) naming the violated
    /// rule.
    pub fn validate(&self) -> Result<(), Error> {
        let invalid = |rule: &str, detail: String| {
            Err(Error::validation_failed("system-migration-invalid", rule, detail))
        };
        if self.version != VERSION {
            return invalid(
                "unsupported version",
                format!("migration.yaml version {} is not {VERSION}", self.version),
            );
        }
        let mut disposition_ids = BTreeSet::new();
        for disposition in &self.dispositions {
            if disposition.id.trim().is_empty() {
                return invalid("id required", "a disposition has an empty `id`".into());
            }
            if !disposition_ids.insert(disposition.id.as_str()) {
                return invalid(
                    "duplicate disposition",
                    format!("disposition `{}` appears twice", disposition.id),
                );
            }
            if disposition.reason.trim().is_empty() {
                return invalid(
                    "reason required",
                    format!("disposition `{}` has an empty `reason`", disposition.id),
                );
            }
        }
        let wave_ids: BTreeSet<&str> = self.waves.iter().map(|wave| wave.id.as_str()).collect();
        if wave_ids.len() != self.waves.len() {
            return invalid("duplicate wave", "a wave id appears twice".into());
        }
        for wave in &self.waves {
            wave.validate(&disposition_ids, &wave_ids)?;
        }
        Ok(())
    }
}

impl Disposition {
    /// Content digest of the canonical YAML encoding (the handoff's
    /// per-disposition `digest`).
    ///
    /// # Errors
    ///
    /// Propagates YAML serialization failures.
    pub fn digest(&self) -> Result<SnapshotId, Error> {
        let yaml = artifacts::atomic::serialise_yaml(self)?;
        Ok(SnapshotId::from_digest(&sha256_hex(yaml.as_bytes())))
    }
}

impl Wave {
    /// Content digest of the canonical YAML encoding (the handoff's
    /// `wave.digest` and per-dependency `digest`).
    ///
    /// # Errors
    ///
    /// Propagates YAML serialization failures.
    pub fn digest(&self) -> Result<SnapshotId, Error> {
        let yaml = artifacts::atomic::serialise_yaml(self)?;
        Ok(SnapshotId::from_digest(&sha256_hex(yaml.as_bytes())))
    }

    /// Every inlined `Item` list, paired with its wire name.
    #[must_use]
    pub fn item_lists(&self) -> [(&'static str, &[Item]); 11] {
        [
            ("preconditions", &self.preconditions),
            ("state-movements", &self.state_movements),
            ("coexistence", &self.coexistence),
            ("cutover", &self.cutover),
            ("rollback", &self.rollback),
            ("operational-readiness", &self.operational_readiness),
            ("acceptance", &self.acceptance),
            ("verification", &self.verification),
            ("conservation", &self.conservation),
            ("gaps", &self.gaps),
            ("assumptions", &self.assumptions),
        ]
    }

    fn validate(&self, dispositions: &BTreeSet<&str>, waves: &BTreeSet<&str>) -> Result<(), Error> {
        let invalid = |rule: &str, detail: String| {
            Err(Error::validation_failed(
                "system-migration-invalid",
                rule,
                format!("wave `{}`: {detail}", self.id),
            ))
        };
        if self.id.trim().is_empty() {
            return Err(Error::validation_failed(
                "system-migration-invalid",
                "id required",
                "a wave has an empty `id`".to_string(),
            ));
        }
        if self.outcome.trim().is_empty() {
            return invalid("outcome required", "the `outcome` is empty".into());
        }
        for predecessor in &self.predecessors {
            if !waves.contains(predecessor.as_str()) {
                return invalid(
                    "unresolved predecessor",
                    format!("predecessor `{predecessor}` is not a wave in this plan"),
                );
            }
            if predecessor == &self.id {
                return invalid("self predecessor", "a wave cannot precede itself".into());
            }
        }
        for reference in &self.dispositions {
            if !dispositions.contains(reference.as_str()) {
                return invalid(
                    "unresolved disposition",
                    format!("disposition `{reference}` is not declared in `dispositions[]`"),
                );
            }
        }
        for (list, items) in self.item_lists() {
            let mut ids = BTreeSet::new();
            for item in items {
                if item.id.trim().is_empty() {
                    return invalid("id required", format!("a `{list}` entry has an empty `id`"));
                }
                if !ids.insert(item.id.as_str()) {
                    return invalid(
                        "duplicate id",
                        format!("`{list}` id `{}` appears twice", item.id),
                    );
                }
            }
        }
        let target_ids: BTreeSet<&str> =
            self.targets.iter().map(|target| target.id.as_str()).collect();
        if target_ids.len() != self.targets.len() {
            return invalid("duplicate target", "a target id appears twice".into());
        }
        for mapping in &self.delivery_mappings {
            if !target_ids.contains(mapping.target.as_str()) {
                return invalid(
                    "unresolved mapping target",
                    format!("mapping target `{}` is not a wave target", mapping.target),
                );
            }
        }
        Ok(())
    }
}

/// One item's content digest (the handoff's per-entry `digest`).
///
/// # Errors
///
/// Propagates YAML serialization failures.
pub fn item_digest(item: &Item) -> Result<SnapshotId, Error> {
    let yaml = artifacts::atomic::serialise_yaml(item)?;
    Ok(SnapshotId::from_digest(&sha256_hex(yaml.as_bytes())))
}
