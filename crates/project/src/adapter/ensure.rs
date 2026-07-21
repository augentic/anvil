//! Component-deployment ensure: the deterministic provisioning kernels
//! behind the shipped provider's [`super::Resolver::ensure_source`] /
//! [`super::Resolver::ensure_target`].
//!
//! A package selector that misses the global single-file store is
//! fetched from the registry configured in `.specify/wasm-pkg.toml`
//! and installed as `<store-root>/<name>@<version>.wasm` plus its
//! digest `.meta` sidecar, then verified after write. A local
//! component selector is validated, canonicalized, and mirrored into
//! the project component cache
//! (`<project-cache>/components/<name>.wasm`) with provenance stamped
//! in [`ComponentMeta`]. A bare selector provisions nothing — the
//! resolver locates the already-seeded cache entry live.
//!
//! Everything here is deterministic; only the byte transport is
//! deployment-specific, so the kernels take `fetch` as a caller
//! closure (the shipped WASI provider sends `wasi:http`, test
//! providers script or refuse).
//!
//! The v1 wire protocol is a plain HTTP GET against a documented
//! registry layout: `<base>/adapters/<namespace>/<name>@<version>.wasm`,
//! where `<base>` is `https://<registry>` unless the configured
//! registry value already carries a scheme.

use std::fs;
use std::future::Future;
use std::path::{Path, PathBuf};

use artifacts::atomic::bytes_write;
use diagnostics::cache::{file_content_digest, verify_store_entry, write_store_meta};
use error::Error;
use serde::{Deserialize, Serialize};

use super::core::{ResolvedSource, ResolvedTarget};
use super::resolver::{Component, Resolver as _, component_cache_entry};
use super::selector::{AdapterSelector, canonicalize_component};
use super::{metadata, selector};
use crate::config::Layout;
use crate::handler::ExecutionPaths;

/// Ensure a source selector for the component deployment: provision
/// (hydrate / mirror), then resolve through the component resolver.
///
/// # Errors
///
/// Provisioning failures (`adapter-hydrate-failed`,
/// `adapter-digest-mismatch`, `adapter-component-missing`,
/// `adapter-canonicalize-failed`) ahead of resolve failures.
pub async fn source<F, Fut>(
    runner: metadata::Runner, selector: &AdapterSelector, paths: &ExecutionPaths,
    now: jiff::Timestamp, fetch: F,
) -> Result<ResolvedSource, Error>
where
    F: Fn(String) -> Fut,
    Fut: Future<Output = Result<Vec<u8>, Error>>,
{
    provision(selector, paths, now, fetch).await?;
    Component::new(runner).resolve_source(selector, paths)
}

/// Ensure a target selector for the component deployment: provision
/// (hydrate / mirror), then resolve through the component resolver.
///
/// # Errors
///
/// Provisioning failures (`adapter-hydrate-failed`,
/// `adapter-digest-mismatch`, `adapter-component-missing`,
/// `adapter-canonicalize-failed`) ahead of resolve failures.
pub async fn target<F, Fut>(
    runner: metadata::Runner, selector: &AdapterSelector, paths: &ExecutionPaths,
    now: jiff::Timestamp, fetch: F,
) -> Result<ResolvedTarget, Error>
where
    F: Fn(String) -> Fut,
    Fut: Future<Output = Result<Vec<u8>, Error>>,
{
    provision(selector, paths, now, fetch).await?;
    Component::new(runner).resolve_target(selector, paths)
}

/// Make one selector resolvable: install a missing package pin into
/// the global store, mirror a local component into the project cache,
/// or nothing for a bare development name.
///
/// The resolve-free provisioning kernel shared by the guest provider's
/// `ensure_*` legs (via [`source`] / [`target`]) and the deployment
/// launcher's pre-run closure hydration, which locates paths itself
/// instead of resolving metadata.
///
/// # Errors
///
/// `adapter-hydrate-failed`, `adapter-digest-mismatch`,
/// `adapter-component-missing`, or `adapter-canonicalize-failed`.
pub async fn provision<F, Fut>(
    selector: &AdapterSelector, paths: &ExecutionPaths, now: jiff::Timestamp, fetch: F,
) -> Result<(), Error>
where
    F: Fn(String) -> Fut,
    Fut: Future<Output = Result<Vec<u8>, Error>>,
{
    match selector {
        AdapterSelector::Bare { .. } => Ok(()),
        AdapterSelector::Package {
            namespace,
            name,
            version,
        } => hydrate(paths, namespace, name, version, fetch).await,
        AdapterSelector::Component { path } => mirror(path, paths, now),
    }
}

/// Install a pinned package into the global single-file store when it
/// is missing: fetch, write the entry, write the digest sidecar, then
/// verify-after-write. A present entry is a no-op.
async fn hydrate<F, Fut>(
    paths: &ExecutionPaths, namespace: &str, name: &str, version: &semver::Version, fetch: F,
) -> Result<(), Error>
where
    F: Fn(String) -> Fut,
    Fut: Future<Output = Result<Vec<u8>, Error>>,
{
    let version = version.to_string();
    let entry = paths.locations().store_entry(name, &version);
    if entry.is_file() {
        return Ok(());
    }

    let url = package_url(paths.project_root(), namespace, name, &version);
    let bytes = fetch(url.clone()).await.map_err(|err| Error::Diag {
        code: "adapter-hydrate-failed",
        detail: format!("failed to hydrate `{namespace}:{name}@{version}` from {url}: {err}"),
    })?;

    fs::create_dir_all(paths.locations().store_root())?;
    bytes_write(&entry, &bytes)?;
    let digest = file_content_digest(&entry);
    let meta = paths.locations().store_meta(name, &version);
    write_store_meta(&meta, &digest, None)?;
    verify_store_entry(&entry, &meta).map_err(|mismatch| {
        super::resolver::digest_mismatch(
            &format!("store entry {}", entry.display()),
            "verify-after-write",
            &mismatch,
        )
    })?;
    Ok(())
}

/// Mirror an operator-supplied local component into the project
/// component cache — the project-local leg the bare/component resolver
/// probes first — stamping provenance in [`ComponentMeta`].
///
/// Carries the persisted-mirror fallback: a component selector stays
/// resolvable after the operator's original file is removed, because
/// the earlier mirror in the project component cache satisfies
/// re-ensure. The explicit [`seed`] verb has no such fallback — a
/// missing path there is an operator mistake to surface, not a
/// re-ensure to satisfy.
fn mirror(path: &Path, paths: &ExecutionPaths, now: jiff::Timestamp) -> Result<(), Error> {
    let absolute =
        if path.is_absolute() { path.to_path_buf() } else { paths.project_root().join(path) };
    if !absolute.is_file()
        && let Ok(name) = selector::name_from_component(path)
        && component_cache_entry(paths, &name).is_file()
    {
        return Ok(());
    }
    seed(path, paths, now).map(drop)
}

/// The seeded identity one [`seed`] produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seeded {
    /// Kebab-case adapter name derived from the component filename.
    pub name: String,
    /// The mirrored project component cache entry.
    pub entry: PathBuf,
    /// The canonical operator-supplied component the entry mirrors.
    pub source: PathBuf,
}

/// Seed one operator-supplied `.wasm` component into the project
/// component cache.
///
/// Canonicalizes, derives the kebab name from the filename, copies to
/// `<project-cache>/components/<name>.wasm`, and stamps per-component
/// provenance ([`ComponentMeta`]). Re-seeding the same name replaces
/// the entry and its sidecar — the explicit operator command is the
/// approval act.
///
/// Axis-neutral by design: adapter names are unique across axes, so
/// the binding that later resolves the bare name supplies the expected
/// axis; a wrong-world component fails at the dispatch/metadata gate,
/// not during seeding. Relative `path`s anchor at the carried project
/// root. The shared mirror kernel behind the component-selector ensure
/// leg ([`mirror`], which adds the persisted-mirror fallback) and
/// `specify adapter add` (strict: a missing path fails even when the
/// derived name is already cached — a stale entry must not mask a
/// typo).
///
/// # Errors
///
/// `adapter-component-missing` when `path` is not a `.wasm` file,
/// `adapter-canonicalize-failed` when it cannot be canonicalized, and
/// I/O failures from the copy or provenance write.
pub fn seed(path: &Path, paths: &ExecutionPaths, now: jiff::Timestamp) -> Result<Seeded, Error> {
    let absolute =
        if path.is_absolute() { path.to_path_buf() } else { paths.project_root().join(path) };
    ensure_component_file(&absolute, &path.display().to_string())?;
    let canonical = canonicalize_component(path, paths.project_root())?;
    let name = selector::name_from_component(&canonical)?;

    let entry = component_cache_entry(paths, &name);
    if let Some(parent) = entry.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(&canonical, &entry)?;

    let meta = ComponentMeta {
        source: format!("file://{}", canonical.display()),
        fetched_at: now.strftime("%Y-%m-%dT%H:%M:%SZ").to_string(),
    };
    let serialised = serde_saphyr::to_string(&meta)?;
    fs::write(ComponentMeta::path(paths, &name), serialised)?;
    Ok(Seeded {
        name,
        entry,
        source: canonical,
    })
}

fn ensure_component_file(path: &Path, original: &str) -> Result<(), Error> {
    if path.is_file() && path.extension().is_some_and(|ext| ext == "wasm") {
        return Ok(());
    }
    Err(Error::Diag {
        code: "adapter-component-missing",
        detail: format!(
            "adapter `{original}` did not resolve to a `.wasm` component file at {} (an \
             adapter is a single WebAssembly component)",
            path.display()
        ),
    })
}

/// Per-component provenance for a mirrored entry under
/// `<project-cache>/components/`.
///
/// The cache tenant carries its own metadata inside its own tree, one
/// sidecar per component so two seeded adapters never clobber each
/// other's provenance.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ComponentMeta {
    /// The adapter source value (a `file://` component URI) the
    /// component cache was populated from.
    pub source: String,
    /// ISO 8601 timestamp of when the component was last mirrored.
    pub fetched_at: String,
}

impl ComponentMeta {
    /// Absolute path to the `<name>.meta.yaml` provenance sidecar
    /// beside the mirrored `<name>.wasm` entry inside the out-of-tree
    /// `<project-cache>/components/` tenant.
    #[must_use]
    pub fn path(paths: &ExecutionPaths, name: &str) -> PathBuf {
        paths.cache_dir().join("components").join(format!("{name}.meta.yaml"))
    }
}

/// The registry fetch URL for one pinned package, from the project's
/// wasm-pkg configuration.
fn package_url(project_dir: &Path, namespace: &str, name: &str, version: &str) -> String {
    let registry = registry_for(project_dir, namespace);
    let base = if registry.contains("://") { registry } else { format!("https://{registry}") };
    format!("{}/adapters/{namespace}/{name}@{version}.wasm", base.trim_end_matches('/'))
}

/// Registry authority the compiled default and a fresh scaffold agree
/// on; `.specify/wasm-pkg.toml` overrides it per project.
const DEFAULT_REGISTRY: &str = "augentic.io";

/// The subset of `.specify/wasm-pkg.toml` hydration reads. Mirrors the
/// wasm-pkg config shape so `wkg --config .specify/wasm-pkg.toml` and
/// the fetch leg agree on namespace routing.
#[derive(Debug, Default, Deserialize)]
struct WasmPkgConfig {
    #[serde(default)]
    default_registry: Option<String>,
    #[serde(default)]
    namespace_registries: std::collections::BTreeMap<String, String>,
}

/// Resolve the registry for `namespace`: the project's
/// `namespace_registries.<namespace>`, then its `default_registry`,
/// then the compiled default. A missing or unparseable config file
/// falls through to the default rather than failing — a fresh init
/// has not written the file yet.
fn registry_for(project_dir: &Path, namespace: &str) -> String {
    let path = Layout::new(project_dir).specify_dir().join(crate::init::WASM_PKG_CONFIG_FILENAME);
    let config: WasmPkgConfig =
        fs::read_to_string(path).ok().and_then(|raw| toml::from_str(&raw).ok()).unwrap_or_default();
    config
        .namespace_registries
        .get(namespace)
        .cloned()
        .or(config.default_registry)
        .unwrap_or_else(|| DEFAULT_REGISTRY.to_string())
}
