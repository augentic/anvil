//! Global content-addressed adapter store (RFC-48 D5).
//!
//! Adapters resolve from a single global store keyed by the immutable
//! `(name, version)` identity — the Cargo `~/.cargo/registry` model.
//! [`install_tofu`] pulls the published artifact once (trust-on-first-use),
//! unpacks it into a temp dir under the store root, makes the tree
//! read-only, renames it into place atomically, and records a
//! verify-on-read sidecar (RFC-48 D4): the entry's deterministic tree
//! content digest plus the registry layer digest for provenance. A file
//! lock around the unpack-rename window makes concurrent installs of one
//! identity idempotent. The store path resolver and the sidecar helpers
//! live on the `specify-schema` leaf
//! ([`specify_schema::cache::adapter_store_entry`],
//! [`specify_schema::cache::verify_store_entry`]) so this install path and
//! the offline resolve/verify path agree on one location and one digest.

use std::fs::{self, File};
use std::path::{Path, PathBuf};

use specify_schema::cache::{self, adapter_store_entry};

use crate::error::ExtensionError;
use crate::oci::{self, RegistryAuth};
use crate::pack;

/// Trust-on-first-use install of an adapter artifact.
///
/// Pulls the artifact from `reference`, materializes it at the immutable
/// store entry for `(name, version)`, and records the verify-on-read
/// sidecar (RFC-48 D4/D5 install-on-fetch).
///
/// The store entry is content-addressed, read-only, and immutable once
/// installed, so its presence is the read-integrity guarantee; the
/// recorded tree-content digest in the sidecar
/// ([`specify_schema::cache::write_store_meta`]) backs cross-machine
/// verify-on-read at resolve time
/// ([`specify_schema::cache::verify_store_entry`]).
///
/// Idempotent: an already-present entry is returned without a re-pull
/// (and without re-recording its sidecar).
///
/// # Errors
///
/// Propagates `adapter-transport-failed` (pull) and `adapter-pack-failed`
/// (unpack / store I/O / sidecar write).
pub fn install_tofu(
    name: &str, version: &str, reference: &str, auth: &RegistryAuth,
) -> Result<PathBuf, ExtensionError> {
    let entry = adapter_store_entry(name, version);
    if entry.is_dir() {
        return Ok(entry);
    }
    let layer = oci::pull_adapter(reference, auth)?;
    install_layer(&entry, &layer)?;
    record_store_meta(name, version, &entry, &layer)?;
    Ok(entry)
}

/// Record the RFC-48 D4 verify-on-read sidecar beside a freshly
/// installed store entry: the deterministic tree content digest the
/// resolver re-checks, plus the registry layer digest for provenance.
///
/// The sidecar is a writable sibling of the read-only entry, so this
/// runs after [`install_layer`] has published and frozen the tree.
///
/// # Errors
///
/// Returns `adapter-pack-failed` when the sidecar cannot be written.
fn record_store_meta(
    name: &str, version: &str, entry: &Path, layer: &[u8],
) -> Result<(), ExtensionError> {
    let tree_digest = cache::tree_content_digest(entry);
    let layer_digest = pack::content_digest(layer);
    cache::write_store_meta(name, version, &tree_digest, Some(&layer_digest)).map_err(|err| {
        ExtensionError::pack(format!("write verify-on-read sidecar for {name}@{version}: {err}"))
    })
}

/// Materialize a verified `layer` at the immutable store `entry` with
/// atomic, idempotent, read-only semantics. Exposed within the crate so
/// the store layout is exercised without a live registry.
///
/// # Errors
///
/// Returns `adapter-pack-failed` when the store root cannot be created,
/// the lock cannot be taken, the layer cannot be unpacked, or the
/// temp-to-entry rename fails.
pub(crate) fn install_layer(entry: &Path, layer: &[u8]) -> Result<(), ExtensionError> {
    let root = entry.parent().ok_or_else(|| {
        ExtensionError::pack(format!("store entry {} has no parent", entry.display()))
    })?;
    let key = entry_key(entry)?;
    fs::create_dir_all(root).map_err(|err| {
        ExtensionError::pack(format!("create store root {}: {err}", root.display()))
    })?;

    // Serialize concurrent installers of this identity behind a sibling
    // lock file. The lock is advisory; the post-lock re-check is the
    // authority, so a peer that won the race makes this call a no-op.
    let lock_path = root.join(format!(".{key}.lock"));
    let lock = File::create(&lock_path).map_err(|err| {
        ExtensionError::pack(format!("create store lock {}: {err}", lock_path.display()))
    })?;
    lock.lock().map_err(|err| {
        ExtensionError::pack(format!("lock store entry {}: {err}", lock_path.display()))
    })?;

    if entry.is_dir() {
        return Ok(());
    }

    let temp = root.join(format!(".{key}.tmp.{}", std::process::id()));
    if temp.exists() {
        fs::remove_dir_all(&temp).map_err(|err| {
            ExtensionError::pack(format!("clear stale temp {}: {err}", temp.display()))
        })?;
    }
    pack::unpack_adapter(layer, &temp)?;
    set_tree_read_only(&temp)?;
    // Atomic publish: a reader either sees the absent entry or the fully
    // materialized one, never a half-written tree.
    fs::rename(&temp, entry).map_err(|err| {
        ExtensionError::pack(format!("publish store entry {}: {err}", entry.display()))
    })?;
    Ok(())
}

/// The immutable `name@version` final component used to derive sibling
/// lock and temp paths.
fn entry_key(entry: &Path) -> Result<String, ExtensionError> {
    entry
        .file_name()
        .and_then(|name| name.to_str())
        .map(ToOwned::to_owned)
        .ok_or_else(|| ExtensionError::pack(format!("store entry {} has no name", entry.display())))
}

/// Recursively mark every file in the installed tree read-only so the
/// content-addressed store entry cannot be mutated in place.
fn set_tree_read_only(dir: &Path) -> Result<(), ExtensionError> {
    for entry in fs::read_dir(dir)
        .map_err(|err| ExtensionError::pack(format!("read store temp {}: {err}", dir.display())))?
    {
        let entry =
            entry.map_err(|err| ExtensionError::pack(format!("read store temp entry: {err}")))?;
        let path = entry.path();
        let meta = fs::symlink_metadata(&path)
            .map_err(|err| ExtensionError::pack(format!("stat {}: {err}", path.display())))?;
        if meta.is_dir() {
            set_tree_read_only(&path)?;
        } else {
            let mut perms = meta.permissions();
            perms.set_readonly(true);
            fs::set_permissions(&path, perms)
                .map_err(|err| ExtensionError::pack(format!("chmod {}: {err}", path.display())))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
