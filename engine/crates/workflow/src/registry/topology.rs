//! `.specify/topology.lock` — the committed projection of each member
//! project's identity.
//!
//! `registry.yaml` carries membership + location only. A project's
//! authored intent (`adapter`, `description`) lives in its
//! `.specify/project.yaml`; its derived identity — the `surface[]` of
//! owned domains and a `recent[]` tail of merge outcomes — is a
//! deterministic structural projection of its baseline
//! (`.specify/specs/` + `.specify/journal.jsonl`). `specify workspace
//! sync` resolves both into this committed lockfile so workspace plan-time
//! topology (`workspace_topology`) reads a single derived source offline. The
//! lockfile is machine-written (write-if-changed, mirroring
//! `.specify/context.lock`); operators never hand-edit it.

use std::collections::HashMap;
use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use serde::{Deserialize, Serialize};
use specify_diagnostics::{Diagnostic, Severity};
use specify_error::Error;
use specify_model::atomic::yaml_write;

use crate::Platform;
use crate::adapter::{PlatformsViolation, TargetAdapter};
use crate::change::plan_finding;
use crate::config::ProjectConfig;
use crate::init::adapter_ref_from_value;
use crate::registry::Registry;

/// Current `topology.lock` schema version.
pub const CURRENT_TOPOLOGY_LOCK_VERSION: u64 = 1;

/// In-memory representation of `.specify/topology.lock`.
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
    /// Target adapter in `name@vN` form, resolved from the project's
    /// `project.yaml.adapter`.
    pub target: String,
    /// Single-sentence domain characterisation from the project's
    /// `project.yaml`. Absent stays off the wire.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Deterministic baseline surface: one entry per
    /// `.specify/specs/<domain>/spec.md`, projected from the slot's
    /// merged baseline. Empty stays off the wire (greenfield).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub surface: Vec<Surface>,
    /// The last `M` `slice.archive.created` outcome summaries from the
    /// slot's journal ledger, in append order. Empty stays off the wire.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recent: Vec<String>,
    /// Accepted Decision Records projected from `.specify/decisions/`,
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Decision {
    /// The durable `DEC-NNNN` id.
    pub id: String,
    /// The record's H1 heading text.
    pub title: String,
    /// Topic slugs this decision governs (RFC-46 D3), projected from the
    /// record's `topics:` front-matter. The plan-time join key against
    /// surveyed lead `topics[]`. Empty stays off the wire.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub topics: Vec<String>,
}

/// One baseline domain's projected surface.
///
/// The domain slug and a bounded sample of its requirement titles. Shared
/// by [`TopologyProject`] and the reconciliation envelope's `ProjectRef`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Surface {
    /// Domain directory slug under `.specify/specs/`.
    pub domain: String,
    /// Requirement-block headings (`Requirement.name`, inline tag
    /// stripped) in `REQ-NNN` id order, capped at
    /// [`super::identity::SURFACE_TITLE_CAP`].
    pub requirements: Vec<String>,
    /// Count of requirement titles elided past the cap. Absent when
    /// the domain fits within the cap.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub more: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct Version {
    version: u64,
}

impl TopologyLock {
    /// Wrap a resolved project list into a versioned lock document.
    #[must_use]
    pub const fn from_projects(projects: Vec<TopologyProject>) -> Self {
        Self {
            version: CURRENT_TOPOLOGY_LOCK_VERSION,
            projects,
        }
    }

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
                ".specify/topology.lock must be a supported version",
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
        crate::schema::validate_topology_lock(&lock)?;
        Ok(Some(lock))
    }

    /// Atomically write the cache, after schema validation.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Validation`] when the lock fails its schema, or
    /// the underlying I/O error on a failed write.
    pub fn save(&self, path: &Path) -> Result<(), Error> {
        crate::schema::validate_topology_lock(self)?;
        yaml_write(path, self)
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
    /// - Any error from [`TargetAdapter::resolve`] when the adapter
    ///   cannot be resolved against the slot.
    pub fn resolve(
        registry_name: &str, config: &ProjectConfig, slot_dir: &Path,
    ) -> Result<Self, Error> {
        let adapter_value = config.adapter.as_deref().ok_or_else(|| {
            Error::validation_failed(
                "topology-cache-project-adapter-missing",
                "workspace slot project.yaml provides a target adapter",
                format!("workspace slot `{registry_name}` project.yaml omits the `adapter` field"),
            )
        })?;
        let resolved = TargetAdapter::resolve(&adapter_ref_from_value(adapter_value), slot_dir)?;
        let target = format!("{}@{}", resolved.manifest.name, resolved.manifest.version);

        validate_topology_platforms(
            registry_name,
            &config.platforms,
            resolved.manifest.platforms.as_ref(),
            &resolved.manifest.name,
        )?;

        let projection = super::identity::project_baseline(slot_dir)?;
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
/// Maps each [`PlatformsViolation`] from the shared
/// [`crate::adapter::PlatformsCapability::check`] kernel onto the
/// `topology-cache-project-platforms-*` diagnostic family.
fn validate_topology_platforms(
    registry_name: &str, platforms: &[Platform],
    capability: Option<&crate::adapter::PlatformsCapability>, target_name: &str,
) -> Result<(), Error> {
    let Some(cap) = capability else {
        return Ok(());
    };

    cap.check(platforms).map_err(|violation| match violation {
        PlatformsViolation::RequiredButMissing { defaults } => Error::validation_failed(
            "topology-cache-project-platforms-missing",
            format!("workspace slot `{registry_name}` declares platforms"),
            format!(
                "workspace slot `{registry_name}` target '{target_name}' requires platforms \
                 but project.yaml declares none; default set is [{}]",
                defaults.join(", "),
            ),
        ),
        PlatformsViolation::MissingCore => Error::validation_failed(
            "topology-cache-project-platforms-must-include-core",
            format!("workspace slot `{registry_name}` platform set includes `core`"),
            format!(
                "workspace slot `{registry_name}` platform set must include `core`; \
                 every project that declares platforms requires the shared Rust core crate",
            ),
        ),
        PlatformsViolation::NotAllowed { platform, allowed } => Error::validation_failed(
            "topology-cache-project-platforms-not-allowed",
            format!("workspace slot `{registry_name}` platform `{platform}` is allowed"),
            format!(
                "workspace slot `{registry_name}` platform `{platform}` is not allowed \
                 by target '{target_name}'; allowed: [{}]",
                allowed.join(", "),
            ),
        ),
    })
}

/// Compare the committed `.specify/topology.lock` against each
/// materialised slot's projection, returning staleness diagnostics.
///
/// Compares the lock against each slot's current `project.yaml` *and
/// baseline projection*
/// (`surface[]` from `.specify/specs/`, `recent[]` from the journal
/// ledger), returning a `topology-cache-stale` suggestion on divergence
/// (the fix is `specify workspace sync`). Because the projection is
/// deterministic, this is a regenerate-and-compare check:
/// [`TopologyProject::resolve`] re-derives the fresh entry and any drift
/// in `target` / `description` / `surface` / `recent` trips the warning.
/// A slot whose topology cannot be re-derived yields a
/// `workspace-slot-config-unreadable` important finding instead.
/// The project's `project.yaml` plus its baseline are authoritative and
/// the cache is the derived projection of them.
///
/// `workspace_base` is the top-level `workspace/`; `topology_lock_path` is
/// `.specify/topology.lock`. The binary handler renders the returned
/// diagnostics — it owns no projection logic of its own.
#[must_use]
pub fn cache_staleness(
    registry: &Registry, workspace_base: &Path, topology_lock_path: &Path,
) -> Vec<Diagnostic> {
    let mut results = Vec::new();
    let lock = TopologyLock::load(topology_lock_path).ok().flatten();
    let cached: HashMap<&str, &TopologyProject> = lock
        .as_ref()
        .map(|lock| lock.projects.iter().map(|p| (p.name.as_str(), p)).collect())
        .unwrap_or_default();

    for rp in &registry.projects {
        let slot_project_dir = workspace_base.join(&rp.name);
        if !slot_project_dir.join(".specify").join("project.yaml").exists() {
            continue;
        }
        let fresh = match ProjectConfig::load(&slot_project_dir)
            .and_then(|cfg| TopologyProject::resolve(&rp.name, &cfg, &slot_project_dir))
        {
            Ok(fresh) => fresh,
            Err(err) => {
                results.push(plan_finding(
                    "workspace-slot-config-unreadable",
                    Severity::Important,
                    format!("workspace slot '{}' topology could not be derived: {err}", rp.name),
                    None,
                ));
                continue;
            }
        };
        let stale = cached.get(rp.name.as_str()).is_none_or(|cached| **cached != fresh);
        if stale {
            results.push(plan_finding(
                "topology-cache-stale",
                Severity::Suggestion,
                format!(
                    "workspace slot '{}' has drifted from .specify/topology.lock; \
                     run `specify workspace sync` to regenerate the topology cache",
                    rp.name
                ),
                None,
            ));
        }
    }
    results
}

fn malformed(detail: String) -> Error {
    Error::validation_failed(
        "topology-lock-malformed",
        ".specify/topology.lock must be a supported topology lock file",
        detail,
    )
}

#[cfg(test)]
mod tests;
