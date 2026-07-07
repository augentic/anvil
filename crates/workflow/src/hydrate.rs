//! RFC-65 hydration kernel.
//!
//! Pinned-ref collection over `project.yaml` and `plan.yaml`, plus the
//! surface-agnostic [`hydrate`] driver that probes the global adapter
//! store, pulls on miss through an injected fetch leg, and verifies
//! each entry's recorded digest.
//!
//! The fetch leg ([`Fetch`]) is injected by the caller: the root
//! `specify` binary composes `registry::store::install_tofu` here, so
//! this crate stays wasmtime- and network-free (the guest never
//! hydrates — a miss without a fetch is the typed
//! `adapter-not-installed`).
//!
//! Cross-machine digest pinning: every resolved entry is verified
//! against the committed `.specify/adapters.lock` ([`AdaptersLock`])
//! when the lock carries its identity; a first install appends the
//! identity's digest and writes the lock back atomically (never in
//! `frozen` mode, which is strictly read-only). The verification half
//! ([`verify_resolved`] + [`verify_locked`]) is shared with the
//! binary's drive-time deployment discovery, so RFC-65 AC8 ("drift
//! aborts before any guest loads") holds on every manifest-producing
//! path, not just the provisioning triggers.

pub mod lock;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub use lock::AdaptersLock;
use specify_error::Error;
use specify_schema::cache::{adapter_store_entry, file_content_digest, read_store_meta};

use crate::change::Plan;
use crate::config::{Layout, ProjectConfig};
use crate::init::{AdapterPackage, recognize_package};

/// The injected fetch leg.
///
/// Pulls one pinned identity into the global store and returns its
/// entry path. The root binary supplies
/// `registry::store::install_tofu`; idempotency and concurrency-safety
/// are the store's (blocking install lock, atomic publish), not the
/// kernel's.
pub type Fetch<'a> = &'a dyn Fn(&AdapterPackage) -> Result<PathBuf, Error>;

/// One hydrated identity in the resolved set.
///
/// Carries everything a lock entry (`.specify/adapters.lock`) or a
/// deployment-manifest guest entry needs — the identity, the immutable
/// store entry path, and the entry's content digest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedAdapter {
    /// Kebab-case adapter name.
    pub name: String,
    /// Exact semver pin (RFC-48 D2 — hydration takes exact pins only).
    pub version: semver::Version,
    /// The single-file store entry `<store-root>/<name>@<version>.wasm`.
    pub path: PathBuf,
    /// `sha256:<hex>` content digest of the entry — the recorded
    /// sidecar digest when present, else recomputed from the bytes.
    pub digest: String,
}

/// Collect every pinned identity a project declares.
///
/// Gathers the `project.yaml.adapter` target pin, the optional
/// `project.yaml.adapters:` prefetch list, and `plan.yaml` source
/// pins when a plan is present. Deduplicated on `(name, version)`,
/// first occurrence wins.
///
/// Bare, unpinned names are not hydration inputs — they keep today's
/// project-local resolution (component cache, then the development
/// release build) and never cross the network.
///
/// # Errors
///
/// - [`Error::NotInitialized`] when `.specify/project.yaml` is absent.
/// - `adapter-prefetch-unpinned` for a `project.yaml.adapters:` entry
///   that is not a pinned package reference or `name@<semver>`
///   shorthand.
/// - `adapter-package-ref-*` for a malformed package reference.
/// - Plan load and parse errors when `plan.yaml` exists but is
///   invalid.
pub fn collect_refs(project_dir: &Path) -> Result<Vec<AdapterPackage>, Error> {
    let config = ProjectConfig::load(project_dir)?;
    let mut refs = config_refs(&config)?;
    refs.extend(plan_refs(project_dir)?);
    Ok(dedupe(refs))
}

/// The `project.yaml` half of [`collect_refs`].
///
/// Gathers the `adapter:` target pin (when it parses as a package
/// reference or versioned shorthand) plus every `adapters:` prefetch
/// entry. This is the set `specify init` hydrates — plan pins join
/// through [`collect_refs`] at the explicit sync trigger.
///
/// Stage D seam: `specify:core@<the binary's own version>` joins this
/// set when the binary does not embed its core (RFC-65 move 4).
///
/// # Errors
///
/// See [`collect_refs`].
pub fn config_refs(config: &ProjectConfig) -> Result<Vec<AdapterPackage>, Error> {
    let mut refs = Vec::new();
    if let Some(adapter) = config.adapter.as_deref()
        && let Some(package) = recognize_package(adapter)
    {
        refs.push(package?);
    }
    for entry in &config.adapters {
        let Some(package) = recognize_package(entry) else {
            return Err(Error::Diag {
                code: "adapter-prefetch-unpinned",
                detail: format!(
                    "`project.yaml.adapters:` entry `{entry}` is not a pinned adapter identity; \
                     each prefetch entry must pin an exact version (`<name>@<semver>` or \
                     `<namespace>:<name>@<semver>`) — hydration never resolves a bare name over \
                     the network"
                ),
            });
        };
        refs.push(package?);
    }
    Ok(dedupe(refs))
}

/// `plan.yaml` source pins: each `sources.<key>.version` joined with
/// its binding's adapter name as a first-party pinned identity. An
/// absent plan contributes nothing; unpinned bindings are skipped.
fn plan_refs(project_dir: &Path) -> Result<Vec<AdapterPackage>, Error> {
    let plan_path = Layout::new(project_dir).plan_path();
    if !plan_path.exists() {
        return Ok(Vec::new());
    }
    let plan = Plan::load(&plan_path)?;
    Ok(plan
        .sources
        .values()
        .filter_map(|binding| {
            binding
                .version
                .clone()
                .map(|version| AdapterPackage::first_party(binding.adapter.clone(), version))
        })
        .collect())
}

/// Hydrate every pinned ref against the global store (RFC-65).
///
/// Per identity: probe the store entry, pull on miss through `fetch`
/// (never when `frozen`), verify the recorded digest, then verify the
/// entry against the committed `.specify/adapters.lock` when the lock
/// carries the identity — appending it (and writing the lock back
/// atomically) when it does not. Returns the resolved set,
/// deduplicated on `(name, version)`, in first-occurrence order.
///
/// `frozen` is strictly read-only: a store miss is the typed
/// `adapter-not-installed` instead of a fetch, and a new-to-the-lock
/// identity is verified but never appended — the committed artifact is
/// untouched.
///
/// An empty `refs` set touches nothing — no lock is read or created.
/// Lock entries for identities no longer declared are left in place
/// (store entries are immutable and shared across projects).
///
/// Non-interactive by construction: no prompt exists at or below this
/// call, and every failure is a typed error naming the identity.
///
/// # Errors
///
/// - `adapter-not-installed` ([`Error::Validation`], exit 2) — a store
///   miss in `frozen` mode; names the identity and the literal
///   `specify adapters sync` command.
/// - `adapter-digest-mismatch` — the entry failed verify-on-read
///   against its recorded sidecar digest (RFC-48 D4), or drifted from
///   the digest committed in `.specify/adapters.lock`; names the
///   identity and both digests.
/// - `adapters-lock-malformed` / `adapters-lock-version-too-new` —
///   the committed lock cannot be read ([`AdaptersLock::load`]).
/// - Any error from the injected `fetch` leg (the `tool-package-*`
///   transport family, `adapter-store-failed`).
pub fn hydrate(
    project_dir: &Path, refs: &[AdapterPackage], frozen: bool, fetch: Fetch<'_>,
) -> Result<Vec<ResolvedAdapter>, Error> {
    if refs.is_empty() {
        return Ok(Vec::new());
    }
    let lock_path = Layout::new(project_dir).adapters_lock_path();
    let mut lock = AdaptersLock::load(&lock_path)?.unwrap_or_default();
    let mut lock_changed = false;
    let mut resolved = Vec::new();
    let mut seen = BTreeSet::new();
    for package in refs {
        let version = package.version.to_string();
        if !seen.insert((package.name.clone(), version.clone())) {
            continue;
        }
        let entry = adapter_store_entry(&package.name, &version);
        let path = if entry.is_file() {
            entry
        } else if frozen {
            return Err(Error::validation_failed(
                "adapter-not-installed",
                "every declared pinned adapter must be installed in the global store",
                format!(
                    "adapter `{}@{version}` is not installed in the global store at {}; frozen \
                     mode fetches nothing — run `specify adapters sync` to hydrate the declared \
                     set",
                    package.name,
                    entry.display()
                ),
            ));
        } else {
            fetch(package)?
        };
        let adapter = verify_resolved(&package.name, &package.version, path)?;
        verify_locked(&lock, &adapter)?;
        if !frozen {
            lock_changed |= pin_locked(&mut lock, &adapter);
        }
        resolved.push(adapter);
    }
    if lock_changed {
        lock.save(&lock_path)?;
    }
    Ok(resolved)
}

/// Verify one resolved entry against the digest committed in
/// `.specify/adapters.lock` when the lock carries its identity.
///
/// The RFC-65 AC8 gate, shared by the hydration kernel and the
/// binary's drive-time deployment discovery. Read-only: pinning a
/// first-install identity is the [`hydrate`] kernel's own concern, so
/// drive-time callers never write the lock.
///
/// # Errors
///
/// `adapter-digest-mismatch` naming the identity and both digests.
pub fn verify_locked(lock: &AdaptersLock, adapter: &ResolvedAdapter) -> Result<(), Error> {
    let identity = format!("{}@{}", adapter.name, adapter.version);
    match lock.adapters.get(&identity) {
        Some(locked) if *locked != adapter.digest => Err(Error::Diag {
            code: "adapter-digest-mismatch",
            detail: format!(
                "adapter `{identity}` store entry at {} does not match the digest committed in \
                 .specify/adapters.lock: locked {locked} but actual {}",
                adapter.path.display(),
                adapter.digest,
            ),
        }),
        _ => Ok(()),
    }
}

/// Pin a new-to-the-lock identity's digest into the in-memory lock.
/// Returns `true` when the lock gained the entry (the kernel writes it
/// back once, after the whole set resolves clean).
fn pin_locked(lock: &mut AdaptersLock, adapter: &ResolvedAdapter) -> bool {
    let identity = format!("{}@{}", adapter.name, adapter.version);
    if lock.adapters.contains_key(&identity) {
        return false;
    }
    lock.adapters.insert(identity, adapter.digest.clone());
    true
}

/// Verify one store entry against its recorded sidecar digest and
/// project it into the resolved set.
///
/// RFC-48 D4 verify-on-read (a missing sidecar fails open), carrying
/// the trustworthy digest [`verify_locked`] compares against the
/// committed pin. Shared with the binary's drive-time deployment
/// discovery, which runs the same projection over each pinned entry
/// it admits.
///
/// # Errors
///
/// `adapter-digest-mismatch` naming the identity and both digests.
pub fn verify_resolved(
    name: &str, version: &semver::Version, path: PathBuf,
) -> Result<ResolvedAdapter, Error> {
    let version_str = version.to_string();
    if let Err(mismatch) = specify_schema::cache::verify_store_entry(name, &version_str) {
        return Err(Error::Diag {
            code: "adapter-digest-mismatch",
            detail: format!(
                "adapter `{name}@{version_str}` store entry at {} failed verify-on-read: \
                 recorded digest {} but recomputed {}",
                path.display(),
                mismatch.recorded,
                mismatch.actual,
            ),
        });
    }
    let digest = read_store_meta(name, &version_str).unwrap_or_else(|| file_content_digest(&path));
    Ok(ResolvedAdapter {
        name: name.to_string(),
        version: version.clone(),
        path,
        digest,
    })
}

/// Deduplicate on `(name, version)`, first occurrence wins.
fn dedupe(refs: Vec<AdapterPackage>) -> Vec<AdapterPackage> {
    let mut seen = BTreeSet::new();
    refs.into_iter()
        .filter(|package| seen.insert((package.name.clone(), package.version.to_string())))
        .collect()
}
