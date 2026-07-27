//! `.emery/topology.lock` — the committed projection of each member
//! project's identity.
//!
//! `registry.yaml` carries membership + location only. A project's
//! authored intent (`adapter`, `description`) lives in its
//! `.emery/project.yaml`; its derived identity — the `surface[]` of
//! owned domains and a `recent[]` tail of merge outcomes — is a
//! deterministic structural projection of its baseline
//! (`.emery/specs/` + `.emery/journal.jsonl`). `emery workspace
//! sync` resolves both into this committed lockfile so workspace plan-time
//! topology (`workspace_topology`) reads a single derived source offline. The
//! lockfile is machine-written (write-if-changed, mirroring
//! `.emery/context.lock`); operators never hand-edit it.

use std::collections::HashMap;
use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use diagnostics::{Diagnostic, Severity};
use error::Error;
use serde::{Deserialize, Serialize};

use crate::Platform;
use crate::adapter::{AdapterSelector, PlatformsSurface, Resolver};
use crate::config::ProjectConfig;
use crate::handler::ExecutionPaths;
use crate::plan::finding;
use crate::registry::Registry;

/// Current `topology.lock` schema version.
pub const CURRENT_TOPOLOGY_LOCK_VERSION: u64 = 1;

/// In-memory representation of `.emery/topology.lock`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TopologyLock {
    /// Schema version. `1` is the only accepted value for this release.
    pub version: u64,
    /// One entry per registry member project, in registry order.
    #[serde(default)]
    pub projects: Vec<TopologyProject>,
}

/// One resolved member project — its authored intent plus the
/// deterministic projection of its baseline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TopologyProject {
    /// Registry slot name — the `plan.yaml.slices[].project` binding
    /// key. Identity stays the registry name, not `project.yaml.name`.
    pub name: String,
    /// Target adapter resolved from the project's
    /// `project.yaml.adapter`: `name@vN` for a pinned identity, bare
    /// `name` for an unpinned cache resolve.
    pub target: String,
    /// Single-sentence domain characterisation from the project's
    /// `project.yaml`. Absent stays off the wire.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Deterministic baseline surface: one entry per
    /// `.emery/specs/<domain>/spec.md`, projected from the slot's
    /// merged baseline. Empty stays off the wire (greenfield).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub surface: Vec<Surface>,
    /// The last `M` `slice.archive.created` outcome summaries from the
    /// slot's journal ledger, in append order. Empty stays off the wire.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recent: Vec<String>,
    /// Accepted Decision Records projected from `.emery/decisions/`,
    /// the most recent `K` in `DEC-NNNN` ascending order.
    /// The third routing-identity axis — *why* the project is shaped the
    /// way it is. Empty stays off the wire.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub decisions: Vec<Decision>,
    /// Count of accepted decisions elided past the `K` cap. Absent when
    /// the catalogue fits within `K`.
    #[serde(default, rename = "decisions-more", skip_serializing_if = "Option::is_none")]
    pub decisions_more: Option<u64>,
    /// Target platforms this project builds for, projected from
    /// `project.yaml.platforms`. Empty stays off the wire (non-platforms
    /// targets omit the field).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub platforms: Vec<Platform>,
}

/// One accepted Decision Record projected into routing identity.
///
/// Title only — no body, `Context`, or `Consequences` prose is
/// projected. Shared by [`TopologyProject`] and the reconciliation
/// envelope's `ProjectRef`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Decision {
    /// The durable `DEC-NNNN` id.
    pub id: String,
    /// The record's H1 heading text.
    pub title: String,
    /// Topic slugs this decision governs, projected from the
    /// record's `topics:` front-matter. The plan-time join key against
    /// surveyed lead `topics[]`. Empty stays off the wire.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub topics: Vec<String>,
}

/// One baseline domain's projected surface.
///
/// The domain slug and a bounded sample of its requirement titles. Shared
/// by [`TopologyProject`] and the reconciliation envelope's `ProjectRef`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Surface {
    /// Domain directory slug under `.emery/specs/`.
    pub domain: String,
    /// Requirement-block headings (`Requirement.name`, inline tag
    /// stripped) in `REQ-NNN` id order, capped at
    /// `super::identity::SURFACE_TITLE_CAP`.
    pub requirements: Vec<String>,
    /// Count of requirement titles elided past the cap. Absent when
    /// the domain fits within the cap.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub more: Option<u64>,
}

/// The topology target-reference grammar.
///
/// `name@version` for a pinned identity, bare `name` for an unpinned
/// cache resolve. The single producer formula both topology writers
/// share; `plan::TargetRef` parses the same grammar back.
#[must_use]
pub fn target_ref(name: &str, version: Option<&semver::Version>) -> String {
    version.map_or_else(|| name.to_string(), |version| format!("{name}@{version}"))
}

#[derive(Debug, Deserialize)]
struct Version {
    version: u64,
}

impl TopologyLock {
    /// Load + version-gate the committed cache. A missing file yields
    /// `Ok(None)` — the registry layer decides whether absence is fatal
    /// (workspace plan-time topology raises `topology-cache-missing`).
    ///
    /// # Errors
    ///
    /// - [`Error::Validation`] `topology-lock-malformed` when the YAML
    ///   does not parse or carries an unsupported version.
    /// - [`Error::Validation`] `topology-lock-version-too-new` when the
    ///   version is newer than this binary supports.
    pub fn load(path: &Path) -> Result<Option<Self>, Error> {
        let contents = match fs::read_to_string(path) {
            Ok(contents) => contents,
            Err(err) if err.kind() == ErrorKind::NotFound => return Ok(None),
            Err(err) => return Err(Error::Io(err)),
        };

        let version: Version = serde_saphyr::from_str(&contents).map_err(|err| {
            malformed(format!("topology-lock-malformed: failed to read lock version: {err}"))
        })?;
        if version.version > CURRENT_TOPOLOGY_LOCK_VERSION {
            return Err(Error::validation_failed(
                "topology-lock-version-too-new",
                ".emery/topology.lock must be a supported version",
                format!(
                    "topology-lock-version-too-new: lock version {} > supported \
                     {CURRENT_TOPOLOGY_LOCK_VERSION}",
                    version.version
                ),
            ));
        }
        if version.version != CURRENT_TOPOLOGY_LOCK_VERSION {
            return Err(malformed(format!(
                "topology-lock-malformed: unsupported lock version {}; expected \
                 {CURRENT_TOPOLOGY_LOCK_VERSION}",
                version.version
            )));
        }

        let lock: Self = serde_saphyr::from_str(&contents)
            .map_err(|err| malformed(format!("topology-lock-malformed: {err}")))?;
        Ok(Some(lock))
    }
}

impl TopologyProject {
    /// Project one materialised slot into a resolved topology entry:
    /// the slot [`ProjectConfig`]'s authored intent (`description`) and
    /// resolved `target`, plus the deterministic baseline projection
    /// (`surface[]` / `recent[]`) read from `slot_dir`.
    /// `registry_name` is the slot/registry name (the binding key);
    /// `slot_dir` is the slot's project directory, used both to resolve
    /// the adapter to its canonical `name@vN` ref and as the baseline
    /// projection root.
    ///
    /// # Errors
    ///
    /// - [`Error::Validation`] `topology-cache-project-adapter-missing`
    ///   when the slot `project.yaml` omits `adapter`.
    /// - [`Error::Validation`] `topology-cache-project-platforms-missing`
    ///   when the resolved target requires platforms but the slot
    ///   declares none.
    /// - [`Error::Validation`] `topology-cache-project-platforms-must-include-core`
    ///   when the slot's platform set omits `Platform::Core`.
    /// - [`Error::Validation`] `topology-cache-project-platforms-not-allowed`
    ///   when a declared platform falls outside the target's allowed set.
    /// - Any error from [`crate::adapter::Resolver::resolve_target`] when the adapter
    ///   cannot be resolved against the slot.
    pub fn resolve(
        resolver: &impl Resolver, registry_name: &str, config: &ProjectConfig,
        slot_paths: &ExecutionPaths,
    ) -> Result<Self, Error> {
        let adapter_value = config.adapter.as_deref().ok_or_else(|| {
            Error::validation_failed(
                "topology-cache-project-adapter-missing",
                "workspace slot project.yaml provides a target adapter",
                format!("workspace slot `{registry_name}` project.yaml omits the `adapter` field"),
            )
        })?;
        let target_adapter =
            resolver.resolve_target(&AdapterSelector::parse(adapter_value)?, slot_paths)?;
        let target =
            target_ref(&target_adapter.manifest.name, target_adapter.manifest.version.as_ref());

        validate_topology_platforms(
            registry_name,
            &config.platforms,
            target_adapter.manifest.platforms.as_ref(),
            &target_adapter.manifest.name,
        )?;

        let projection = super::identity::project_baseline(slot_paths.project_root())?;
        Ok(Self {
            name: registry_name.to_string(),
            target,
            description: config.description.clone(),
            surface: projection.surface,
            recent: projection.recent,
            decisions: projection.decisions,
            decisions_more: projection.decisions_more,
            platforms: config.platforms.clone(),
        })
    }
}

/// Backstop validation of a workspace slot's platforms against the
/// resolved target adapter's [`crate::adapter::PlatformsCapability`].
/// Violations map onto the `topology-cache-project-platforms-*`
/// diagnostic family via the shared
/// the shared platform-validation error converter.
fn validate_topology_platforms(
    registry_name: &str, platforms: &[Platform],
    capability: Option<&crate::adapter::PlatformsCapability>, target_name: &str,
) -> Result<(), Error> {
    let Some(cap) = capability else {
        return Ok(());
    };

    cap.check(platforms).map_err(|violation| {
        violation.into_error(PlatformsSurface::TopologySlot {
            registry: registry_name,
            target: target_name,
        })
    })
}

/// Compare the committed `.emery/topology.lock` against each
/// materialised slot's projection, returning staleness diagnostics.
///
/// Compares the lock against each slot's current `project.yaml` *and
/// baseline projection*
/// (`surface[]` from `.emery/specs/`, `recent[]` from the journal
/// ledger), returning a `topology-cache-stale` suggestion on divergence.
/// Because the projection is deterministic, this is a regenerate-and-compare check:
/// [`TopologyProject::resolve`] re-derives the fresh entry and any drift
/// in `target` / `description` / `surface` / `recent` trips the warning.
/// A slot whose topology cannot be re-derived yields a
/// `workspace-slot-config-unreadable` important finding instead.
/// The project's `project.yaml` plus its baseline are authoritative and
/// the cache is the derived projection of them.
///
/// `workspace_base` is the top-level `workspace/`; `topology_lock_path` is
/// `.emery/topology.lock`. The binary handler renders the returned
/// diagnostics — it owns no projection logic of its own.
#[must_use]
pub fn cache_staleness(
    resolver: &impl Resolver, registry: &Registry, paths: &ExecutionPaths,
    topology_lock_path: &Path,
) -> Vec<Diagnostic> {
    let workspace_base = paths.project_root().join("workspace");
    let mut results = Vec::new();
    let lock = TopologyLock::load(topology_lock_path).ok().flatten();
    let cached: HashMap<&str, &TopologyProject> = lock
        .as_ref()
        .map(|lock| lock.projects.iter().map(|p| (p.name.as_str(), p)).collect())
        .unwrap_or_default();

    for rp in &registry.projects {
        let slot_project_dir = workspace_base.join(&rp.name);
        if !slot_project_dir.join(".emery").join("project.yaml").exists() {
            continue;
        }
        let slot_paths = paths.with_root(&slot_project_dir);
        let config = match ProjectConfig::load(&slot_project_dir) {
            Ok(config) => config,
            Err(err) => {
                results.push(finding(
                    "workspace-slot-config-unreadable",
                    Severity::Important,
                    format!("workspace slot '{}' topology could not be derived: {err}", rp.name),
                    None,
                ));
                continue;
            }
        };
        let fresh = match TopologyProject::resolve(resolver, &rp.name, &config, &slot_paths) {
            Ok(fresh) => fresh,
            Err(err) => {
                results.push(finding(
                    "workspace-slot-config-unreadable",
                    Severity::Important,
                    format!("workspace slot '{}' topology could not be derived: {err}", rp.name),
                    None,
                ));
                continue;
            }
        };
        if let Some(unresolvable) = slot_binding_unresolvable(resolver, &rp.name, &config, paths) {
            results.push(unresolvable);
        }
        let stale = cached.get(rp.name.as_str()).is_none_or(|cached| **cached != fresh);
        if stale {
            results.push(finding(
                "topology-cache-stale",
                Severity::Suggestion,
                format!(
                    "workspace slot '{}' has drifted from .emery/topology.lock; \
                     the topology cache must be regenerated before planning",
                    rp.name
                ),
                None,
            ));
        }
    }
    results
}

/// The slot-binding reach check under ID-only guest resolution: the
/// routed adapter id carries no slot, so a slot target dispatches only
/// through an exact store pin or an identity resolvable at the
/// deployment project's root (its seeded component cache, or a
/// natively linked adapter). A binding that resolves slot-locally but
/// not at the deployment root would pass topology derivation and then
/// fail at dispatch — surface it here instead.
///
/// Package pins are exempt: the host resolver installs a store miss
/// during dispatch (pull-on-miss), so a pin is always reachable by id.
fn slot_binding_unresolvable(
    resolver: &impl Resolver, registry_name: &str, config: &ProjectConfig, paths: &ExecutionPaths,
) -> Option<Diagnostic> {
    let value = config.adapter.as_deref()?;
    let selector = match AdapterSelector::parse(value) {
        // An exact pin installs by id at dispatch; an unparseable
        // value has already failed topology derivation.
        Ok(AdapterSelector::Package { .. }) | Err(_) => return None,
        Ok(selector) => selector,
    };
    if resolver.resolve_target(&selector, paths).is_ok() {
        return None;
    }
    let name = selector.name().ok()?;
    Some(finding(
        "workspace-slot-binding-unresolvable",
        Severity::Important,
        format!(
            "workspace slot '{registry_name}' binds target `{value}`, which is not resolvable by \
             routed adapter id (the id carries no slot): pin an exact published version \
             (`emery:{name}@<semver>`) or seed the component into the deployment project's \
             cache (`emery adapter add <path/to/{name}.wasm>` at the deployment root)"
        ),
        None,
    ))
}

fn malformed(detail: String) -> Error {
    Error::validation_failed(
        "topology-lock-malformed",
        ".emery/topology.lock must be a supported topology lock file",
        detail,
    )
}
