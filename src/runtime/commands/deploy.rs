//! Deployment assembly for the generated manifest (RFC-65).
//!
//! Shared by the two provisioning triggers (`specify init`, `specify
//! adapters sync`) and the guest leg: [`regenerate`] discovers the
//! full deployment set — bound adapters through the axis resolvers,
//! `project.yaml.adapters:` prefetch pins against the global store,
//! and the component-cache scan for unbound local components — stages
//! the embedded workflow guest into the deployment tenant, and hands
//! the set to the pure generator in `specify_workflow::deploy`. No
//! leg here fetches: a pinned identity absent from the store is the
//! typed `adapter-not-installed` naming the identity and the literal
//! sync command (the guest never hydrates).
//!
//! Every pinned entry the discovery admits is digest-verified twice:
//! RFC-48 D4 verify-on-read against the store sidecar, then RFC-65
//! AC8 against the committed `.specify/adapters.lock` when the lock
//! carries the identity (`hydrate::{verify_resolved, verify_locked}`)
//! — so a warm-but-divergent store aborts with the typed
//! `adapter-digest-mismatch` before any manifest is written or guest
//! driven. The lock is read-only here; only the hydration kernel
//! appends to it.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use specify_error::Error;
use specify_workflow::adapter::{AdapterRef, Axis, SourceAdapter, TargetAdapter};
use specify_workflow::change::Plan;
use specify_workflow::config::{Layout, ProjectConfig};
use specify_workflow::deploy::{self, DeployGuest};
use specify_workflow::hydrate::{self, AdaptersLock};
use specify_workflow::init::adapter_ref_from_value;

/// What a bare-name adapter that resolves nowhere does to the
/// regeneration.
///
/// The provisioning triggers skip it — bare names are the development
/// posture, hydration never fetches them, and the guest leg
/// regenerates strictly before every drive anyway. The guest leg
/// fails loudly: a bound adapter missing at drive time must surface
/// as the resolver's typed diagnostic, never a silently thinner
/// deployment. Pinned identities are strict in both modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BareMiss {
    /// Skip the unresolvable bare name (provisioning triggers).
    Skip,
    /// Propagate the resolver's `adapter-not-found` (the guest leg).
    Fail,
}

/// Regenerate the deployment manifest for `project_dir` and return its
/// path (`<project-cache>/deployment/omnia.toml`).
///
/// Runs the full discovery, stages the embedded workflow guest into
/// the deployment tenant (skipping the write when the staged bytes
/// already match), and generates the manifest atomically.
pub fn regenerate(project_dir: &Path, bare_miss: BareMiss) -> Result<PathBuf, Error> {
    // The manifest lives out-of-tree, so every path it carries must be
    // absolute — including the mount, which callers may pass as ".".
    let project_dir = fs::canonicalize(project_dir).map_err(Error::Io)?;
    let adapters = discover(&project_dir, bare_miss)?;
    let core = stage_core(&project_dir)?;
    deploy::generate(&project_dir, &core, &adapters)
}

/// Materialise the embedded workflow guest at the deployment tenant's
/// core staging path, skipping the write when the bytes already match
/// (Stage D swaps this staging for a hydrated `specify:core` store
/// entry or a macro embed without touching the generator).
fn stage_core(project_dir: &Path) -> Result<PathBuf, Error> {
    let path = deploy::core_stage_path(project_dir);
    let bytes = specify_runtime::WORKFLOW_GUEST_WASM;
    if fs::read(&path).is_ok_and(|current| current == bytes) {
        return Ok(path);
    }
    specify_model::atomic::bytes_write(&path, bytes)?;
    Ok(path)
}

/// Discover the full deployment set with the resolvers' precedence.
///
/// Three legs, first hit per `(axis, name)` winning:
///
/// 1. **Bound adapters through the axis resolvers.** The target bound
///    in `project.yaml` (`adapter:`) and each source bound in
///    `plan.yaml` (`sources.<key>.adapter`) resolve through
///    [`TargetAdapter::resolve`] / [`SourceAdapter::resolve`]. A
///    pinned identity is pre-probed against the global store so a
///    miss is the typed `adapter-not-installed` (never a fetch); a
///    bare name that resolves nowhere follows `bare_miss`.
/// 2. **`project.yaml.adapters:` prefetch pins** against their store
///    entries, with the axis sniffed from the component's exports —
///    prefetched identities are declared but unbound, so there is no
///    axis resolver to consult.
/// 3. **Component-cache scan for unbound local components** (`plan
///    author` runs before `plan.yaml` binds sources): every `*.wasm`
///    in the project component cache, axis-sniffed; a file exporting
///    neither axis interface is skipped, not an error.
///
/// Every pinned entry admitted by legs 1 and 2 passes the shared
/// verification pair from the hydration kernel — D4 verify-on-read
/// plus the committed-lock gate (RFC-65 AC8) — before it can reach the
/// manifest.
fn discover(project_dir: &Path, bare_miss: BareMiss) -> Result<Vec<DeployGuest>, Error> {
    let mut guests = Vec::new();
    let mut seen: BTreeSet<(&'static str, String)> = BTreeSet::new();
    let config = load_config(project_dir)?;
    let lock =
        AdaptersLock::load(&Layout::new(project_dir).adapters_lock_path())?.unwrap_or_default();

    for (axis, adapter_ref) in bound_adapters(config.as_ref(), project_dir)? {
        if !seen.insert((axis.dir_segment(), adapter_ref.name.clone())) {
            continue;
        }
        if let Some(version) = adapter_ref.version.as_ref() {
            let entry =
                specify_schema::cache::adapter_store_entry(&adapter_ref.name, &version.to_string());
            if !entry.is_file() {
                return Err(deploy::adapter_not_installed(&adapter_ref.name, version, &entry));
            }
        }
        let resolved = match axis {
            Axis::Source => SourceAdapter::resolve(&adapter_ref, project_dir)
                .map(|adapter| adapter.location.path().clone()),
            Axis::Target => TargetAdapter::resolve(&adapter_ref, project_dir)
                .map(|adapter| adapter.location.path().clone()),
        };
        let component = match resolved {
            Ok(component) => component,
            Err(Error::Diag {
                code: "adapter-not-found",
                ..
            }) if bare_miss == BareMiss::Skip && adapter_ref.version.is_none() => continue,
            Err(err) => return Err(err),
        };
        if let Some(version) = adapter_ref.version.as_ref() {
            // The resolver's D4 verify-on-read does not consult the
            // committed lock, so a warm-but-divergent store (populated
            // by another project or machine) is caught here — before
            // any manifest is written or guest driven (RFC-65 AC8).
            let entry = hydrate::verify_resolved(&adapter_ref.name, version, component.clone())?;
            hydrate::verify_locked(&lock, &entry)?;
        }
        guests.push(DeployGuest {
            axis,
            name: adapter_ref.name,
            version: adapter_ref.version,
            component,
        });
    }

    if let Some(config) = config.as_ref() {
        for package in hydrate::config_refs(config)? {
            let version = package.version.to_string();
            let entry = specify_schema::cache::adapter_store_entry(&package.name, &version);
            if !entry.is_file() {
                return Err(deploy::adapter_not_installed(&package.name, &package.version, &entry));
            }
            // D4 verify-on-read plus the committed-lock gate — the
            // same checks the bound-adapter leg gets, so both pinned
            // legs are symmetric (a prefetch pin is never admitted on
            // presence alone).
            let verified =
                hydrate::verify_resolved(&package.name, &package.version, entry.clone())?;
            hydrate::verify_locked(&lock, &verified)?;
            let Some(axis) = sniffed_axis(&entry) else {
                continue;
            };
            if seen.insert((axis.dir_segment(), package.name.clone())) {
                guests.push(DeployGuest {
                    axis,
                    name: package.name,
                    version: Some(package.version),
                    component: entry,
                });
            }
        }
    }

    for (name, component) in cached_components(project_dir) {
        let Some(axis) = sniffed_axis(&component) else {
            continue;
        };
        if seen.insert((axis.dir_segment(), name.clone())) {
            guests.push(DeployGuest {
                axis,
                name,
                version: None,
                component,
            });
        }
    }
    Ok(guests)
}

/// The project config when `.specify/project.yaml` exists — a bare
/// directory binds nothing and the guest reports its own
/// `not-initialized`.
fn load_config(project_dir: &Path) -> Result<Option<ProjectConfig>, Error> {
    if !Layout::new(project_dir).config_path().is_file() {
        return Ok(None);
    }
    ProjectConfig::load(project_dir).map(Some)
}

/// The `(axis, AdapterRef)` pairs the project binds: the `project.yaml`
/// target adapter (skipped for adapter-less workspaces) and every
/// `plan.yaml` source binding.
fn bound_adapters(
    config: Option<&ProjectConfig>, project_dir: &Path,
) -> Result<Vec<(Axis, AdapterRef)>, Error> {
    let mut bound = Vec::new();
    if let Some(value) = config.and_then(|config| config.adapter.as_deref()) {
        bound.push((Axis::Target, adapter_ref_from_value(value)));
    }
    let plan_path = Layout::new(project_dir).plan_path();
    if plan_path.is_file() {
        let plan = Plan::load(&plan_path)?;
        for binding in plan.sources.values() {
            bound.push((
                Axis::Source,
                AdapterRef {
                    name: binding.adapter.clone(),
                    version: binding.version.clone(),
                },
            ));
        }
    }
    Ok(bound)
}

/// The component's exported axis, or `None` when the file is not a
/// component or exports neither axis interface (skipped, not an
/// error).
fn sniffed_axis(component: &Path) -> Option<Axis> {
    match specify_runtime::describe::sniff_axis(component).ok()?? {
        specify_runtime::describe::DescribeAxis::Source => Some(Axis::Source),
        specify_runtime::describe::DescribeAxis::Target => Some(Axis::Target),
    }
}

/// The `(name, component)` pairs in the project component cache
/// (`<project-cache>/components/<name>.wasm`), name-sorted for a
/// deterministic manifest. An absent or unreadable cache is simply
/// empty — adapter components are optional per verb.
fn cached_components(project_dir: &Path) -> Vec<(String, PathBuf)> {
    let root = specify_workflow::adapter::component_cache_dir(project_dir);
    let Ok(entries) = fs::read_dir(&root) else {
        return Vec::new();
    };
    let mut found: Vec<(String, PathBuf)> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            let name = path.file_stem()?.to_str()?.to_owned();
            (path.extension().is_some_and(|ext| ext == "wasm") && path.is_file())
                .then_some((name, path))
        })
        .collect();
    found.sort();
    found
}
