//! `handoffs/<digest>.yaml` — the immutable, content-addressed
//! projection of one migration wave (RFC-104 D10). A handoff carries
//! identities and closed digest references, never copied architecture
//! prose; historical handoffs are never deleted.

use std::collections::BTreeSet;
use std::path::Path;

use diagnostics::digest::sha256_hex;
use error::Error;
use project::snapshot::SnapshotId;
use serde::{Deserialize, Serialize};

use crate::coverage::{Coverage, Disposition as CoverageDisposition};
use crate::decision::Decision;
use crate::layout::Layout;
use crate::migration::{Migration, Wave, item_digest};
use crate::model::Model;
use crate::scope::Scope;

/// The one supported handoff schema version.
const VERSION: u32 = 1;

/// One canonical wave handoff.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Handoff {
    /// Schema version; only `1` is accepted.
    pub version: u32,
    /// The declared engagement identity (`scope.yaml.id`).
    pub definition: String,
    /// Covered `scope.yaml` digest.
    pub scope_digest: SnapshotId,
    /// Covered `coverage.yaml` digest.
    pub coverage_digest: SnapshotId,
    /// Covered `system.yaml` digest.
    pub system_model_digest: SnapshotId,
    /// Covered `migration.yaml` digest.
    pub migration_plan_digest: SnapshotId,
    /// Digest of the sorted `{ id, digest }` catalogue over every
    /// `decisions/<id>.yaml` (absent directory = empty list).
    pub decisions_digest: SnapshotId,
    /// The projected wave.
    pub wave: HandoffWave,
}

/// The projected wave body.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct HandoffWave {
    /// The wave's id in `migration.yaml`.
    pub id: String,
    /// Canonical digest of the wave record.
    pub digest: SnapshotId,
    /// The bounded outcome.
    pub outcome: String,
    /// Named states before and after the wave.
    pub architecture: ArchitectureRefs,
    /// Proposed delivery targets, adapter identity copied exactly as
    /// declared (a name is never resolved to a pin).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub targets: Vec<TargetScope>,
    /// The selected surface leads with their extraction provenance.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence_scopes: Vec<EvidenceScopeRef>,
    /// Reviewed source-to-target assignments.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub delivery_mappings: Vec<DeliveryMappingRef>,
    /// Elements that may experience an observable consequence.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub affected_elements: Vec<String>,
    /// Elements in the delivery ownership envelope.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub touched_elements: Vec<String>,
    /// Read-only architectural context.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub context_elements: Vec<String>,
    /// Predecessor waves.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<Ref>,
    /// External preconditions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub preconditions: Vec<Ref>,
    /// Enacted modernization dispositions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dispositions: Vec<Ref>,
    /// State movement and reconciliation.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub state_movements: Vec<Ref>,
    /// Coexistence requirements.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub coexistence: Vec<Ref>,
    /// Cutover requirements.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cutover: Vec<Ref>,
    /// Rollback position.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rollback: Vec<Ref>,
    /// Operational-readiness requirements.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub operational_readiness: Vec<Ref>,
    /// The acceptance boundary.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub acceptance: Vec<Ref>,
    /// Verification expectations.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub verification: Vec<Ref>,
    /// Conservation expectations.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conservation: Vec<Ref>,
    /// Material unknowns.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gaps: Vec<Ref>,
    /// Commercial assumptions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub assumptions: Vec<Ref>,
    /// Authority decisions (`decisions/<id>.yaml` records only).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub decisions: Vec<Ref>,
}

/// One `{ id, digest }` reference into the reviewed definition.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Ref {
    /// The referenced record's id.
    pub id: String,
    /// Canonical digest of the referenced record.
    pub digest: SnapshotId,
}

/// The named-state references a wave moves between.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ArchitectureRefs {
    /// The named state before the wave.
    pub before: Ref,
    /// The named state after the wave.
    pub after: Ref,
}

/// One proposed delivery target, identity copied as declared.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TargetScope {
    /// Reviewed logical target id.
    pub id: String,
    /// Mutable origin locator.
    pub locator: String,
    /// Operator-declared adapter identity, verbatim.
    pub adapter: String,
}

/// One selected surface lead with its extraction provenance.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct EvidenceScopeRef {
    /// Coverage-row source key.
    pub source: String,
    /// The row's mutable origin locator.
    pub location: String,
    /// The row's operator-declared adapter identity, verbatim.
    pub adapter: String,
    /// The surveyed lead id.
    pub lead: String,
    /// Canonical digest of the persisted Evidence document.
    pub evidence_digest: SnapshotId,
    /// Observed tree the claims were read from (provenance, never the
    /// delivery source pin).
    pub observed_cid: SnapshotId,
    /// Git commit reported by the fetch, when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_revision: Option<String>,
}

/// One reviewed source-to-target assignment.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct DeliveryMappingRef {
    /// Coverage-row source key.
    pub source: String,
    /// The surveyed lead id.
    pub lead: String,
    /// The receiving wave target id.
    pub target: String,
}

/// One projected handoff plus its content address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Projected {
    /// Canonical digest of the handoff — its filename and identity.
    pub digest: SnapshotId,
    /// The projected document.
    pub handoff: Handoff,
}

/// The unresolved-reference fail-closed error.
fn unresolved(detail: String) -> Error {
    Error::validation_failed(
        "system-handoff-unresolved",
        "every handoff reference resolves",
        detail,
    )
}

/// The sorted `{ id, digest }` catalogue over every decision record.
///
/// # Errors
///
/// Propagates YAML serialization failures.
pub fn decisions_catalogue(decisions: &[Decision]) -> Result<Vec<Ref>, Error> {
    let mut catalogue = Vec::with_capacity(decisions.len());
    for decision in decisions {
        catalogue.push(Ref {
            id: decision.id.clone(),
            digest: decision.digest()?,
        });
    }
    catalogue.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(catalogue)
}

/// Digest of the sorted decisions catalogue (absent directory = the
/// empty list's digest).
///
/// # Errors
///
/// Propagates YAML serialization failures.
pub fn decisions_digest(decisions: &[Decision]) -> Result<SnapshotId, Error> {
    let catalogue = decisions_catalogue(decisions)?;
    let yaml = artifacts::atomic::serialise_yaml(&catalogue)?;
    Ok(SnapshotId::from_digest(&sha256_hex(yaml.as_bytes())))
}

/// Deterministically project one wave into its canonical handoff.
///
/// # Errors
///
/// `system-handoff-unresolved` when the wave's architecture states,
/// element ids, disposition or decision references, or evidence
/// scopes do not resolve against the live definition; propagates
/// digest and I/O failures.
pub fn project(
    layout: &Layout<'_>, scope: &Scope, coverage: &Coverage, model: &Model,
    migration: &Migration, decisions: &[Decision], wave: &Wave,
) -> Result<Projected, Error> {
    let architecture = architecture_refs(model, wave)?;
    let handoff = Handoff {
        version: VERSION,
        definition: scope.id.clone(),
        scope_digest: scope.digest()?,
        coverage_digest: coverage.digest()?,
        system_model_digest: model.digest()?,
        migration_plan_digest: migration.digest()?,
        decisions_digest: decisions_digest(decisions)?,
        wave: HandoffWave {
            id: wave.id.clone(),
            digest: wave.digest()?,
            outcome: wave.outcome.clone(),
            architecture,
            targets: wave
                .targets
                .iter()
                .map(|target| TargetScope {
                    id: target.id.clone(),
                    locator: target.locator.clone(),
                    adapter: target.adapter.clone(),
                })
                .collect(),
            evidence_scopes: evidence_scopes(layout, coverage, wave)?,
            delivery_mappings: wave
                .delivery_mappings
                .iter()
                .map(|mapping| DeliveryMappingRef {
                    source: mapping.source.clone(),
                    lead: mapping.lead.clone(),
                    target: mapping.target.clone(),
                })
                .collect(),
            affected_elements: element_refs(model, wave, &wave.affected_elements, "affected")?,
            touched_elements: element_refs(model, wave, &wave.touched_elements, "touched")?,
            context_elements: element_refs(model, wave, &wave.context_elements, "context")?,
            dependencies: dependency_refs(migration, wave)?,
            preconditions: item_refs(&wave.preconditions)?,
            dispositions: disposition_refs(migration, wave)?,
            state_movements: item_refs(&wave.state_movements)?,
            coexistence: item_refs(&wave.coexistence)?,
            cutover: item_refs(&wave.cutover)?,
            rollback: item_refs(&wave.rollback)?,
            operational_readiness: item_refs(&wave.operational_readiness)?,
            acceptance: item_refs(&wave.acceptance)?,
            verification: item_refs(&wave.verification)?,
            conservation: item_refs(&wave.conservation)?,
            gaps: item_refs(&wave.gaps)?,
            assumptions: item_refs(&wave.assumptions)?,
            decisions: decision_refs(decisions, wave)?,
        },
    };
    let yaml = artifacts::atomic::serialise_yaml(&handoff)?;
    let digest = SnapshotId::from_digest(&sha256_hex(yaml.as_bytes()));
    Ok(Projected { digest, handoff })
}

/// Persist one projected handoff at its content address. Historical
/// handoffs are never deleted; rewriting the same digest is
/// byte-idempotent.
///
/// # Errors
///
/// Atomic-write failures.
pub fn write(layout: &Layout<'_>, projected: &Projected) -> Result<(), Error> {
    let yaml = artifacts::atomic::serialise_yaml(&projected.handoff)?;
    let path = layout.handoff_path(projected.digest.digest());
    artifacts::atomic::bytes_write(&path, yaml.as_bytes())
}

/// Load one handoff and verify its content address matches its
/// filename.
///
/// # Errors
///
/// - `Error::YamlDe` for malformed YAML or unknown fields.
/// - `system-handoff-corrupt` when the recomputed digest differs from
///   the digest the filename claims (generated files are engine-owned;
///   a direct edit is staleness, not an amendment).
pub fn load(path: &Path) -> Result<Projected, Error> {
    let text = std::fs::read_to_string(path).map_err(Error::Io)?;
    let handoff: Handoff = serde_saphyr::from_str(&text)?;
    let yaml = artifacts::atomic::serialise_yaml(&handoff)?;
    let digest = SnapshotId::from_digest(&sha256_hex(yaml.as_bytes()));
    let stem = path.file_stem().and_then(|stem| stem.to_str()).unwrap_or_default();
    if digest.digest() != stem {
        return Err(Error::Diag {
            code: "system-handoff-corrupt",
            detail: format!(
                "{} does not match its content digest {}; handoffs are engine-generated — \
                 re-run `emery system plan`",
                path.display(),
                digest.as_str()
            ),
        });
    }
    Ok(Projected { digest, handoff })
}

/// Load every `handoffs/<digest>.yaml`, sorted by digest. An absent
/// directory is the valid empty set.
///
/// # Errors
///
/// Per-file failures per [`load`], plus directory I/O failures.
pub fn load_all(layout: &Layout<'_>) -> Result<Vec<Projected>, Error> {
    let entries = match std::fs::read_dir(layout.handoffs_dir()) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return Err(Error::Io(err)),
    };
    let mut projected = Vec::new();
    for entry in entries {
        let path = entry.map_err(Error::Io)?.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("yaml") {
            continue;
        }
        projected.push(load(&path)?);
    }
    projected.sort_by(|a, b| a.digest.as_str().cmp(b.digest.as_str()));
    Ok(projected)
}

/// Resolve the wave's before/after states against `system.yaml`.
fn architecture_refs(model: &Model, wave: &Wave) -> Result<ArchitectureRefs, Error> {
    let resolve = |name: &str| {
        let state = model.state(name).ok_or_else(|| {
            unresolved(format!(
                "wave `{}`: architecture state `{name}` is not a named state in system.yaml",
                wave.id
            ))
        })?;
        Ok::<Ref, Error>(Ref {
            id: name.to_string(),
            digest: state.digest()?,
        })
    };
    Ok(ArchitectureRefs {
        before: resolve(&wave.architecture.before)?,
        after: resolve(&wave.architecture.after)?,
    })
}

/// Check element ids against the wave's endpoint states.
fn element_refs(
    model: &Model, wave: &Wave, ids: &[String], role: &str,
) -> Result<Vec<String>, Error> {
    let mut members = BTreeSet::new();
    for name in [&wave.architecture.before, &wave.architecture.after] {
        if let Some(state) = model.state(name) {
            members.extend(state.elements.iter().map(|element| element.id.as_str()));
        }
    }
    for id in ids {
        if !members.contains(id.as_str()) {
            return Err(unresolved(format!(
                "wave `{}`: {role} element `{id}` is not an element of `{}` or `{}`",
                wave.id, wave.architecture.before, wave.architecture.after
            )));
        }
    }
    Ok(ids.to_vec())
}

/// Resolve predecessor waves to `{ id, digest }` refs.
fn dependency_refs(migration: &Migration, wave: &Wave) -> Result<Vec<Ref>, Error> {
    let mut refs = Vec::with_capacity(wave.predecessors.len());
    for predecessor in &wave.predecessors {
        let record = migration.wave(predecessor).ok_or_else(|| {
            unresolved(format!(
                "wave `{}`: predecessor `{predecessor}` is not a wave in migration.yaml",
                wave.id
            ))
        })?;
        refs.push(Ref {
            id: record.id.clone(),
            digest: record.digest()?,
        });
    }
    Ok(refs)
}

/// Resolve enacted dispositions to `{ id, digest }` refs.
fn disposition_refs(migration: &Migration, wave: &Wave) -> Result<Vec<Ref>, Error> {
    let mut refs = Vec::with_capacity(wave.dispositions.len());
    for id in &wave.dispositions {
        let record = migration.disposition(id).ok_or_else(|| {
            unresolved(format!(
                "wave `{}`: disposition `{id}` is not declared in migration.yaml",
                wave.id
            ))
        })?;
        refs.push(Ref {
            id: record.id.clone(),
            digest: record.digest()?,
        });
    }
    Ok(refs)
}

/// Resolve named decision records to `{ id, digest }` refs.
fn decision_refs(decisions: &[Decision], wave: &Wave) -> Result<Vec<Ref>, Error> {
    let mut refs = Vec::with_capacity(wave.decisions.len());
    for id in &wave.decisions {
        let record = decisions.iter().find(|decision| &decision.id == id).ok_or_else(|| {
            unresolved(format!(
                "wave `{}`: decision `{id}` has no decisions/{id}.yaml record",
                wave.id
            ))
        })?;
        refs.push(Ref {
            id: record.id.clone(),
            digest: record.digest()?,
        });
    }
    Ok(refs)
}

/// Digest one inlined item list into `{ id, digest }` refs.
fn item_refs(items: &[crate::migration::Item]) -> Result<Vec<Ref>, Error> {
    let mut refs = Vec::with_capacity(items.len());
    for item in items {
        refs.push(Ref {
            id: item.id.clone(),
            digest: item_digest(item)?,
        });
    }
    Ok(refs)
}

/// Close each selected lead over its coverage row and persisted
/// Evidence document.
fn evidence_scopes(
    layout: &Layout<'_>, coverage: &Coverage, wave: &Wave,
) -> Result<Vec<EvidenceScopeRef>, Error> {
    let mut scopes = Vec::with_capacity(wave.evidence_scopes.len());
    for scope in &wave.evidence_scopes {
        let row = coverage.row(&scope.source).ok_or_else(|| {
            unresolved(format!(
                "wave `{}`: evidence scope source `{}` has no coverage row",
                wave.id, scope.source
            ))
        })?;
        if row.disposition != CoverageDisposition::Included {
            return Err(unresolved(format!(
                "wave `{}`: evidence scope source `{}` is not `included`",
                wave.id, scope.source
            )));
        }
        let adapter = row.adapter.clone().unwrap_or_default();
        let observed_cid = row.observed_cid.clone().ok_or_else(|| {
            unresolved(format!(
                "wave `{}`: source `{}` has no observed tree — no successful survey has \
                 completed for it",
                wave.id, scope.source
            ))
        })?;
        let path = layout.evidence_path(&scope.source, &scope.lead);
        let text = std::fs::read_to_string(&path).map_err(|_missing| {
            unresolved(format!(
                "wave `{}`: no persisted Evidence at evidence/{}/{}.yaml",
                wave.id, scope.source, scope.lead
            ))
        })?;
        let document: artifacts::evidence::Document = serde_saphyr::from_str(&text)?;
        let yaml = artifacts::atomic::serialise_yaml(&document)?;
        scopes.push(EvidenceScopeRef {
            source: scope.source.clone(),
            location: row.location.clone(),
            adapter,
            lead: scope.lead.clone(),
            evidence_digest: SnapshotId::from_digest(&sha256_hex(yaml.as_bytes())),
            observed_cid,
            observed_revision: row.observed_revision.clone(),
        });
    }
    Ok(scopes)
}
