//! Canonical slice-local planning projections (RFC-91 D4).
//!
//! The `entry` / `leads` / `decomposition` digests cover only data
//! reachable from the leaf's own entry — unrelated branches never move them.

use std::collections::{BTreeMap, BTreeSet};

use artifacts::discovery::Lead;
use artifacts::evidence::ClaimKind;
use diagnostics::digest::sha256_hex;
use error::Error;
use serde::Serialize;

use super::model::{Disagreement, Divergence, Entry, Plan};
use crate::name::SliceName;
use crate::snapshot::SnapshotId;

/// Wire version stamped into every projection's canonical bytes.
const PROJECTION_VERSION: u32 = 1;

/// The three per-leaf planning digests recorded by a refinement
/// manifest (`inputs.planning`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Projections {
    /// Digest of the leaf's complete plan entry: name, target binding,
    /// resolved source bindings, and ordered dependency identities.
    pub entry: SnapshotId,
    /// Digest of exactly the retained contributing lead closure —
    /// the full lead blocks the entry's source bindings reference.
    pub leads: SnapshotId,
    /// Digest of the decomposition scope. Pre-RFC-88 this is the
    /// canonical single-node projection: the leaf as its own terminal
    /// with empty ancestry and the transitive `depends-on` closure.
    pub decomposition: SnapshotId,
}

impl Projections {
    /// Compute the three canonical projections for `entry`.
    ///
    /// `contributing` is the leaf's resolved contributing lead closure
    /// in binding order (see [`contributing_leads`]); `target` is the
    /// project's *declared* adapter reference from `project.yaml`
    /// (name plus version pin when pinned) — the declared string only,
    /// never a resolved identity, so the projection stays pure.
    ///
    /// # Errors
    ///
    /// `plan-projection-source-unbound` when an entry binding names a
    /// key absent from `plan.sources`; YAML serialization failures.
    pub fn compute(
        plan: &Plan, entry: &Entry, contributing: &[Lead], target: Option<&str>,
    ) -> Result<Self, Error> {
        Ok(Self {
            entry: digest(&entry_projection(plan, entry, target)?)?,
            leads: digest(&LeadsProjection {
                version: PROJECTION_VERSION,
                slice: entry.name.as_str().to_string(),
                leads: contributing.to_vec(),
            })?,
            decomposition: digest(&DecompositionProjection {
                version: PROJECTION_VERSION,
                terminal: entry.name.as_str().to_string(),
                depends_on: dependency_closure(plan, entry),
            })?,
        })
    }
}

/// Resolve the entry's contributing lead closure, in binding order.
///
/// `inventory` is the full `discovery.md` lead set. A bare binding's
/// lead falls back to the owning slice's name (workflow
/// §`Slice.sources`).
///
/// # Errors
///
/// `discovery-lead-unknown` when a bound `(source, lead)` pair has no
/// matching inventory block.
pub fn contributing_leads(entry: &Entry, inventory: &[Lead]) -> Result<Vec<Lead>, Error> {
    entry
        .sources
        .iter()
        .map(|binding| {
            let source = binding.source();
            let lead = binding.lead(entry.name.as_str());
            inventory
                .iter()
                .find(|block| block.source == source && block.lead == lead)
                .cloned()
                .ok_or_else(|| Error::Diag {
                    code: "discovery-lead-unknown",
                    detail: format!(
                        "no lead `{lead}` for source `{source}` in discovery.md; slice `{}` \
                         binds it as a contributing lead",
                        entry.name
                    ),
                })
        })
        .collect()
}

/// Digest of a projection's canonical bytes: the YAML serialization of
/// its DTO (stable serde field order, absent-as-canonical-empty
/// optional fields), hashed as `sha256:<64 hex>`.
fn digest<T: Serialize>(value: &T) -> Result<SnapshotId, Error> {
    let bytes = artifacts::atomic::serialise_yaml(value)?;
    Ok(SnapshotId::from_digest(&sha256_hex(bytes.as_bytes())))
}

/// Canonical `entry` projection bytes. Owned clones of the entry's
/// fields plus the plan-level binding declaration (minus the content
/// `cid`, which the manifest records separately under
/// `inputs.sources`). Every optional field uses absent-as-canonical-
/// empty encoding, so later additions with the same encoding are
/// digest-neutral until they carry data.
#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct EntryProjection {
    version: u32,
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    project: Option<String>,
    /// Declared target adapter reference from `project.yaml` —
    /// rebinding or re-pinning the target stales every manifest.
    #[serde(skip_serializing_if = "Option::is_none")]
    target: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    depends_on: Vec<SliceName>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    sources: Vec<BindingProjection>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    context: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    divergence: Option<Divergence>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    disagreements: Vec<Disagreement>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    authority_override: BTreeMap<ClaimKind, String>,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    allow_composition_replace: bool,
}

/// One resolved `(source, lead)` binding plus the plan-level binding
/// declaration it references.
#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct BindingProjection {
    source: String,
    /// Resolved lead id — the bare-string shorthand encodes its
    /// fallback (the slice name) so authoring sugar never moves bytes.
    lead: String,
    adapter: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    adapter_version: Option<semver::Version>,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<String>,
}

/// Canonical `leads` projection bytes: the full retained lead blocks
/// in binding order.
#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct LeadsProjection {
    version: u32,
    slice: String,
    leads: Vec<Lead>,
}

/// Canonical `decomposition` projection bytes — pre-RFC-88 the leaf is
/// its own terminal.
#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct DecompositionProjection {
    version: u32,
    terminal: String,
    /// Sorted transitive `depends-on` closure from `plan.yaml`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    depends_on: Vec<SliceName>,
}

fn entry_projection(
    plan: &Plan, entry: &Entry, target: Option<&str>,
) -> Result<EntryProjection, Error> {
    let sources = entry
        .sources
        .iter()
        .map(|binding| {
            let key = binding.source();
            let Some(bound) = plan.sources.get(key) else {
                return Err(Error::Diag {
                    code: "plan-projection-source-unbound",
                    detail: format!(
                        "slice `{}` binds source `{key}` which is absent from plan.yaml.sources",
                        entry.name
                    ),
                });
            };
            Ok(BindingProjection {
                source: key.to_string(),
                lead: binding.lead(entry.name.as_str()).to_string(),
                adapter: bound.adapter.clone(),
                adapter_version: bound.version.clone(),
                path: bound.path.clone(),
                value: bound.value.clone(),
            })
        })
        .collect::<Result<Vec<_>, Error>>()?;
    Ok(EntryProjection {
        version: PROJECTION_VERSION,
        name: entry.name.as_str().to_string(),
        project: entry.project.clone(),
        target: target.map(str::to_string),
        depends_on: entry.depends_on.clone(),
        sources,
        context: entry.context.clone(),
        description: entry.description.clone(),
        divergence: entry.divergence,
        disagreements: entry.disagreements.clone(),
        authority_override: entry.authority_override.by_kind.clone(),
        allow_composition_replace: entry.allow_composition_replace,
    })
}

/// Sorted transitive closure of `entry.depends_on` over the plan's
/// entries. Names without a matching entry still enter the closure as
/// identities; cycles terminate through the visited set.
fn dependency_closure(plan: &Plan, entry: &Entry) -> Vec<SliceName> {
    let by_name: BTreeMap<&str, &Entry> =
        plan.entries.iter().map(|e| (e.name.as_str(), e)).collect();
    let mut closure: BTreeSet<SliceName> = BTreeSet::new();
    let mut frontier: Vec<SliceName> = entry.depends_on.clone();
    while let Some(name) = frontier.pop() {
        if name.as_str() == entry.name.as_str() || !closure.insert(name.clone()) {
            continue;
        }
        if let Some(predecessor) = by_name.get(name.as_str()) {
            frontier.extend(predecessor.depends_on.iter().cloned());
        }
    }
    closure.into_iter().collect()
}
