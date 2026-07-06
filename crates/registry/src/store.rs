//! Global content-addressed adapter store (RFC-48 D5, RFC-64 one
//! component).
//!
//! Adapters resolve from a single global store keyed by the immutable
//! `(name, version)` identity — the Cargo `~/.cargo/registry` model.
//! Post-RFC-64 a store entry is one file,
//! `<store-root>/<name>@<version>.wasm`: the published WebAssembly
//! component pulled through the **wasm-pkg** transport
//! (the crate-private `package` module, the same client the `tools[]`
//! resolver uses).
//! [`install_tofu`] pulls the component once (trust-on-first-use),
//! stages it beside the entry, makes it read-only, renames it into
//! place atomically, and records a verify-on-read sidecar (RFC-48 D4):
//! the component's byte digest. A file lock around the stage-rename
//! window makes concurrent installs of one identity idempotent. The
//! store path resolver and the sidecar helpers live on the
//! `specify-schema` leaf ([`specify_schema::cache::adapter_store_entry`],
//! [`specify_schema::cache::verify_store_entry`]) so this install path
//! and the offline resolve/verify path agree on one location and one
//! digest.

use std::fs::{self, File};
use std::path::{Path, PathBuf};

use specify_extension::PackageRequest;
use specify_schema::cache::{self, adapter_store_entry};

use crate::error::ExtensionError;
use crate::package;

/// Trust-on-first-use install of an adapter component.
///
/// Pulls the `namespace:name@version` package through the wasm-pkg
/// transport (honouring the project's `.specify/wasm-pkg.toml`
/// namespace mappings when `project_dir` is set), materializes the
/// component at the immutable store entry for `(name, version)`, and
/// records the verify-on-read sidecar (RFC-48 D4/D5 install-on-fetch).
///
/// The store entry is content-addressed, read-only, and immutable once
/// installed, so its presence is the read-integrity guarantee; the
/// recorded byte digest in the sidecar
/// ([`specify_schema::cache::write_store_meta`]) backs cross-machine
/// verify-on-read at resolve time
/// ([`specify_schema::cache::verify_store_entry`]).
///
/// Idempotent: an already-present entry is returned without a re-pull
/// (and without re-recording its sidecar).
///
/// # Errors
///
/// Propagates the wasm-pkg fetch errors (`tool-package-*` family) and
/// `adapter-store-failed` (store I/O / sidecar write).
pub fn install_tofu(
    namespace: &str, name: &str, version: &str, project_dir: &Path,
) -> Result<PathBuf, ExtensionError> {
    let entry = adapter_store_entry(name, version);
    if entry.is_file() {
        return Ok(entry);
    }
    let request = PackageRequest {
        namespace: namespace.to_string(),
        name: name.to_string(),
        version: version.to_string(),
    };
    let acquired = package::fetch(project_dir, &request, &entry)?;
    install_component(&entry, acquired.temp.path())?;
    record_store_meta(name, version, &entry, &acquired.sha256)?;
    Ok(entry)
}

/// Record the RFC-48 D4 verify-on-read sidecar beside a freshly
/// installed store entry: the component's deterministic byte digest the
/// resolver re-checks, doubling as the registry content digest (the
/// wasm-pkg release content is the component bytes themselves).
///
/// The sidecar is a writable sibling of the read-only entry, so this
/// runs after [`install_component`] has published and frozen the file.
///
/// # Errors
///
/// Returns `adapter-store-failed` when the sidecar cannot be written.
fn record_store_meta(
    name: &str, version: &str, entry: &Path, pulled_sha256: &str,
) -> Result<(), ExtensionError> {
    let digest = cache::file_content_digest(entry);
    let registry_digest = format!("sha256:{pulled_sha256}");
    cache::write_store_meta(name, version, &digest, Some(&registry_digest)).map_err(|err| {
        ExtensionError::store_io(format!(
            "write verify-on-read sidecar for {name}@{version}: {err}"
        ))
    })
}

/// Materialize a pulled component file at the immutable store `entry`
/// with atomic, idempotent, read-only semantics. Exposed within the
/// crate so the store layout is exercised without a live registry.
///
/// # Errors
///
/// Returns `adapter-store-failed` when the store root cannot be created,
/// the lock cannot be taken, the staged copy fails, or the
/// temp-to-entry rename fails.
pub(crate) fn install_component(entry: &Path, component: &Path) -> Result<(), ExtensionError> {
    let root = entry.parent().ok_or_else(|| {
        ExtensionError::store_io(format!("store entry {} has no parent", entry.display()))
    })?;
    let key = entry_key(entry)?;
    fs::create_dir_all(root).map_err(|err| {
        ExtensionError::store_io(format!("create store root {}: {err}", root.display()))
    })?;

    // Serialize concurrent installers of this identity behind a sibling
    // lock file. The lock is advisory; the post-lock re-check is the
    // authority, so a peer that won the race makes this call a no-op.
    let lock_path = root.join(format!(".{key}.lock"));
    let lock = File::create(&lock_path).map_err(|err| {
        ExtensionError::store_io(format!("create store lock {}: {err}", lock_path.display()))
    })?;
    lock.lock().map_err(|err| {
        ExtensionError::store_io(format!("lock store entry {}: {err}", lock_path.display()))
    })?;

    if entry.is_file() {
        return Ok(());
    }

    let temp = root.join(format!(".{key}.tmp.{}", std::process::id()));
    if temp.exists() {
        fs::remove_file(&temp).map_err(|err| {
            ExtensionError::store_io(format!("clear stale temp {}: {err}", temp.display()))
        })?;
    }
    fs::copy(component, &temp).map_err(|err| {
        ExtensionError::store_io(format!("stage component at {}: {err}", temp.display()))
    })?;
    set_read_only(&temp)?;
    // Atomic publish: a reader either sees the absent entry or the fully
    // materialized one, never a half-written file.
    fs::rename(&temp, entry).map_err(|err| {
        ExtensionError::store_io(format!("publish store entry {}: {err}", entry.display()))
    })?;
    Ok(())
}

/// The immutable `name@version.wasm` final component used to derive
/// sibling lock and temp paths.
fn entry_key(entry: &Path) -> Result<String, ExtensionError> {
    entry.file_name().and_then(|name| name.to_str()).map(ToOwned::to_owned).ok_or_else(|| {
        ExtensionError::store_io(format!("store entry {} has no name", entry.display()))
    })
}

/// Mark the staged component read-only so the content-addressed store
/// entry cannot be mutated in place.
fn set_read_only(file: &Path) -> Result<(), ExtensionError> {
    let meta = fs::metadata(file)
        .map_err(|err| ExtensionError::store_io(format!("stat {}: {err}", file.display())))?;
    let mut perms = meta.permissions();
    perms.set_readonly(true);
    fs::set_permissions(file, perms)
        .map_err(|err| ExtensionError::store_io(format!("chmod {}: {err}", file.display())))
}

#[cfg(test)]
mod tests;
