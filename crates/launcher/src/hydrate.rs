//! Pre-run provisioning, component-path resolution, and fail-closed
//! store verification — one pass per invocation.
//!
//! Reuses the ensure kernels (`project::adapter::ensure::provision`):
//! a package pin missing from the global store is hydrated from the
//! registry configured in `.specify/wasm-pkg.toml`; a local component
//! selector is mirrored into the project component cache; a bare name
//! provisions nothing. Paths then resolve through the same probes the
//! engine guest uses (`project::adapter::resolver::locate`), so the
//! deployment enumerates exactly the components in-guest resolution
//! will find. `locate`'s verify-on-read is the single digest pass per
//! store component; resolution here additionally refuses a store
//! entry without a digest sidecar ([`require_sidecar`]) — the
//! launcher is the last gate before the runtime executes the bytes.

use std::path::PathBuf;

use error::Error;
use project::adapter::{ensure, resolver};
use project::handler::ExecutionPaths;

use crate::closure::{Closure, ENGINE_NAME, Requirement};

/// A closure entry with its component file resolved and verified.
#[derive(Debug)]
pub struct ResolvedAdapter {
    pub guest_id: String,
    pub component: PathBuf,
}

/// The fully resolved, verified closure the deployment assembles from.
#[derive(Debug)]
pub struct ResolvedClosure {
    pub engine_component: PathBuf,
    pub adapters: Vec<ResolvedAdapter>,
}

/// Ceiling on establishing a registry connection.
const FETCH_CONNECT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

/// Ceiling on one whole registry GET — a hung registry must not hang
/// the invocation.
const FETCH_TOTAL_TIMEOUT: std::time::Duration = std::time::Duration::from_mins(1);

/// The default registry byte transport: one HTTPS GET per miss,
/// deadline-bounded.
pub async fn http_fetch(url: String) -> Result<Vec<u8>, Error> {
    let failed = |detail: String| Error::Diag {
        code: "adapter-hydrate-failed",
        detail,
    };
    let client = reqwest::Client::builder()
        .connect_timeout(FETCH_CONNECT_TIMEOUT)
        .timeout(FETCH_TOTAL_TIMEOUT)
        .build()
        .map_err(|err| failed(format!("building the registry HTTP client failed: {err}")))?;
    let response =
        client.get(&url).send().await.map_err(|err| failed(format!("GET {url} failed: {err}")))?;
    let status = response.status();
    if !status.is_success() {
        return Err(failed(format!("GET {url} answered {status}")));
    }
    let bytes =
        response.bytes().await.map_err(|err| failed(format!("reading {url} failed: {err}")))?;
    Ok(bytes.to_vec())
}

/// Provision every closure entry (hydrate store misses, mirror local
/// components), resolve each to its component file, and verify every
/// store resolve fail closed. Requirements resolve against the
/// invocation's one carried [`ExecutionPaths`] by default; a
/// workspace-slot requirement re-anchors at its slot directory
/// (`Requirement::root`) — the same layout, the slot's own project
/// root — so slot-local bare and component bindings probe the slot's
/// cache, exactly as the guest resolves them.
///
/// A slot-rooted binding without a resolvable artifact (an unpinned
/// bare or component selector with an empty slot cache) is skipped
/// rather than failing the launch: the guest degrades the same slot to
/// its `workspace-slot-config-unreadable` finding without dispatching
/// it, so the closure-superset invariant is preserved.
pub async fn resolve<F, Fut>(
    paths: &ExecutionPaths, closure: Closure, fetch: F,
) -> Result<ResolvedClosure, Error>
where
    F: Fn(String) -> Fut + Send + Sync,
    Fut: Future<Output = Result<Vec<u8>, Error>> + Send,
{
    let now = jiff::Timestamp::now();

    let engine_version =
        closure.engine.version().expect("the engine selector is a package pin").to_string();
    let engine_component = paths.locations().store_entry(ENGINE_NAME, &engine_version);
    // The binary can carry its own guest component: embedded bytes
    // seed (or refresh) the store entry without a registry fetch;
    // registry hydration remains the fallback for builds without the
    // embed.
    match closure.engine_bytes {
        Some(bytes) => seed_engine(paths, &engine_version, bytes)?,
        None => ensure::provision(&closure.engine, paths, now, &fetch).await?,
    }
    if !engine_component.is_file() {
        return Err(Error::Diag {
            code: "adapter-not-found",
            detail: format!(
                "the engine guest `specify:engine@{engine_version}` is not installed at {}; \
                 hydration from the configured registry did not produce it",
                engine_component.display(),
            ),
        });
    }
    // The engine bypasses `locate`, so it takes its single digest
    // pass here: sidecar presence, then verify-on-read.
    require_sidecar(paths, ENGINE_NAME, &engine_version)?;
    let engine_meta = paths.locations().store_meta(ENGINE_NAME, &engine_version);
    diagnostics::cache::verify_store_entry(&engine_component, &engine_meta).map_err(
        |mismatch| {
            resolver::digest_mismatch(
                &format!("store entry {}", engine_component.display()),
                "verify-on-read",
                &mismatch,
            )
        },
    )?;

    let mut adapters: Vec<ResolvedAdapter> = Vec::new();
    for requirement in closure.adapters {
        let requirement_paths = requirement
            .root
            .as_ref()
            .map_or_else(|| paths.clone(), |slot_dir| paths.with_root(slot_dir));
        ensure::provision(&requirement.selector, &requirement_paths, now, &fetch).await?;
        let resolved = match locate(&requirement, &requirement_paths) {
            Ok(resolved) => resolved,
            Err(Error::Diag {
                code: "adapter-not-found",
                ..
            }) if requirement.root.is_some() => continue,
            Err(err) => return Err(err),
        };
        // `locate` already digest-verified the store entry; the
        // launcher only adds the fail-closed sidecar-presence gate.
        if let Some(version) = requirement.selector.version() {
            require_sidecar(&requirement_paths, &requirement.name, &version.to_string())?;
        }
        merge(&mut adapters, resolved)?;
    }

    Ok(ResolvedClosure {
        engine_component,
        adapters,
    })
}

/// Seed — or refresh — the embedded engine component in the global
/// store: write the entry plus its digest sidecar, the same state a
/// registry hydration leaves behind, so verification and every later
/// invocation flow through the store identically.
///
/// The embedded bytes are authoritative for the binary's own version:
/// a dev rebuild changes the guest without bumping the version, so an
/// entry whose content drifts from the embedded bytes is re-seeded
/// rather than kept — the one sanctioned overwrite of a store entry
/// (release pins never drift; every release carries a new version). A
/// matching entry is a no-op.
fn seed_engine(paths: &ExecutionPaths, version: &str, bytes: &[u8]) -> Result<(), Error> {
    let entry = paths.locations().store_entry(ENGINE_NAME, version);
    let meta = paths.locations().store_meta(ENGINE_NAME, version);
    let digest = format!("sha256:{}", diagnostics::digest::sha256_hex(bytes));
    if diagnostics::cache::read_store_meta(&meta).is_some_and(|recorded| recorded == digest)
        && diagnostics::cache::file_content_digest(&entry) == digest
    {
        return Ok(());
    }
    std::fs::create_dir_all(paths.locations().store_root())?;
    artifacts::atomic::bytes_write(&entry, bytes)?;
    diagnostics::cache::write_store_meta(&meta, &digest, None)?;
    Ok(())
}

/// Fail closed on a store entry without a digest sidecar.
///
/// Deliberately stricter than the engine's fail-open
/// `verify_store_entry` (which waves legacy sidecar-less entries
/// through): the launcher is the last gate before the runtime executes
/// the bytes, so an unverifiable store install is refused. Non-store
/// resolves are exempt — cache-backed and operator-local components
/// are not content-addressed store installs.
fn require_sidecar(paths: &ExecutionPaths, name: &str, version: &str) -> Result<(), Error> {
    let meta = paths.locations().store_meta(name, version);
    if diagnostics::cache::read_store_meta(&meta).is_none() {
        return Err(Error::Diag {
            code: "adapter-sidecar-missing",
            detail: format!(
                "store entry {} has no digest sidecar; the launcher refuses unverifiable \
                 components — reinstall `specify:{name}@{version}` to record one",
                paths.locations().store_entry(name, version).display(),
            ),
        });
    }
    Ok(())
}

fn locate(requirement: &Requirement, paths: &ExecutionPaths) -> Result<ResolvedAdapter, Error> {
    let location =
        resolver::locate(requirement.axis, &requirement.selector, &requirement.name, paths)?;
    Ok(ResolvedAdapter {
        guest_id: requirement.guest_id(),
        component: location.path().clone(),
    })
}

/// Fold one resolved entry into the deployment set: exact duplicates
/// collapse; the same guest id backed by two different component files
/// is unresolvable (the engine dispatches by id alone) and fails
/// closed.
fn merge(adapters: &mut Vec<ResolvedAdapter>, entry: ResolvedAdapter) -> Result<(), Error> {
    match adapters.iter().find(|existing| existing.guest_id == entry.guest_id) {
        None => {
            adapters.push(entry);
            Ok(())
        }
        Some(existing) if existing.component == entry.component => Ok(()),
        Some(existing) => Err(Error::Diag {
            code: "deployment-adapter-conflict",
            detail: format!(
                "adapter `{}` resolves to two different components in one invocation ({} and \
                 {}); the deployment dispatches by adapter id alone, so align the project, \
                 plan, and argv references on one identity",
                entry.guest_id,
                existing.component.display(),
                entry.component.display(),
            ),
        }),
    }
}
