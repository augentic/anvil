//! The filesystem backing of the storage capabilities.
//!
//! [`Disk`] implements [`omnia_guest::StateStore`] / [`omnia_guest::BlobStore`]
//! and preserves the pre-seam on-disk layout byte-for-byte until the WASI
//! host bindings land (design/portable-storage.md step 2). Every
//! engine-owned read or write routes through the capability traits;
//! this module is the only place the engine touches its state on disk.

use std::future::{Future, ready};
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use omnia_guest::{BlobStore, CasError, ContainerMetadata, ObjectMetadata, StateStore};

use crate::handler::{ADAPTERS_CONTAINER, CACHE_MOUNT, Locations, STORE_CONTAINER};
use crate::home::SPEC_CONTAINER;

// The engine-state directory under the project root; keyvalue keys
// resolve beneath it, so `spec/current` lands at `.emery/spec/current`
// — inside the same tree the `spec` blob container is rooted at.
const STATE_DIR: &str = ".emery";

/// Filesystem storage rooted at the deployed preopens (or an explicit
/// directory in tests), reproducing the pre-seam layout exactly.
///
/// Only the operations the engine calls have filesystem bodies; the
/// rest of the capability surface fails typed (`unsupported`) so a new
/// call site cannot silently widen this backing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Disk {
    project: PathBuf,
    cache: PathBuf,
    store: PathBuf,
}

impl Disk {
    /// The deployed layout: `.` is the project-root mount, the cache
    /// root is the named cache preopen, and the store root is the
    /// nominal (unmounted) global store — identical strings on wasm32
    /// (preopen table) and native (invocation directory).
    #[must_use]
    pub fn deployed() -> Self {
        Self {
            project: PathBuf::from("."),
            cache: Locations.cache_dir(),
            store: Locations.store_root().to_path_buf(),
        }
    }

    /// Everything under one explicit root — the test constructor.
    #[must_use]
    pub fn rooted(dir: &Path) -> Self {
        Self {
            project: dir.to_path_buf(),
            cache: dir.join(CACHE_MOUNT),
            store: dir.join("emery-store"),
        }
    }

    fn key_path(&self, key: &str) -> Result<PathBuf> {
        validate(key)?;
        Ok(self.project.join(STATE_DIR).join(key))
    }

    fn container_root(&self, container: &str) -> Result<PathBuf> {
        match container {
            SPEC_CONTAINER => Ok(self.project.join(STATE_DIR).join("spec")),
            ADAPTERS_CONTAINER => Ok(self.cache.join("components")),
            STORE_CONTAINER => Ok(self.store.clone()),
            other => bail!("unknown storage container `{other}`"),
        }
    }

    fn object_path(&self, container: &str, name: &str) -> Result<PathBuf> {
        validate(name)?;
        Ok(self.container_root(container)?.join(name))
    }

    fn state_get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        read_optional(&self.key_path(key)?)
    }

    fn state_set(&self, key: &str, value: &[u8], ttl: Option<u64>) -> Result<Option<Vec<u8>>> {
        if ttl.is_some() {
            bail!("TTL entries are unsupported by the filesystem storage backing");
        }
        let path = self.key_path(key)?;
        let previous = read_optional(&path)?;
        write_atomic(&path, value)?;
        Ok(previous)
    }

    fn state_delete(&self, key: &str) -> Result<()> {
        remove_if_present(&self.key_path(key)?)
    }

    fn state_cas(&self, key: &str, expected: Option<&[u8]>, value: &[u8]) -> Result<(), CasError> {
        let store_err = |err: anyhow::Error| CasError::Store(format!("{err:#}"));
        let path = self.key_path(key).map_err(store_err)?;
        let observed = read_optional(&path).map_err(store_err)?;
        if observed.as_deref() != expected {
            return Err(CasError::Conflict(observed));
        }
        write_atomic(&path, value).map_err(store_err)
    }

    fn blob_get(&self, container: &str, name: &str) -> Result<Option<Vec<u8>>> {
        read_optional(&self.object_path(container, name)?)
    }

    fn blob_put(&self, container: &str, name: &str, data: &[u8]) -> Result<()> {
        write_atomic(&self.object_path(container, name)?, data)
    }

    // Delete the object, then drop any directories the removal left
    // empty (directories are not objects; a pruned generation must not
    // leave its empty directory behind).
    fn blob_delete(&self, container: &str, name: &str) -> Result<()> {
        let root = self.container_root(container)?;
        let path = root.join(validated(name)?);
        remove_if_present(&path)?;
        let mut parent = path.parent();
        while let Some(dir) = parent {
            if dir == root || std::fs::remove_dir(dir).is_err() {
                break;
            }
            parent = dir.parent();
        }
        Ok(())
    }

    fn blob_has(&self, container: &str, name: &str) -> Result<bool> {
        Ok(self.object_path(container, name)?.is_file())
    }

    // Every file under the container root by `/`-separated relative
    // path; a container that was never written lists empty.
    fn blob_list(&self, container: &str) -> Result<Vec<String>> {
        let root = self.container_root(container)?;
        let mut names = Vec::new();
        if root.is_dir() {
            walk(&root, &root, &mut names)?;
        }
        names.sort();
        Ok(names)
    }
}

fn validated(reference: &str) -> Result<&Path> {
    validate(reference)?;
    Ok(Path::new(reference))
}

// Reject absolute, empty, and traversal segments, as the blobstore
// backings do.
fn validate(reference: &str) -> Result<()> {
    let escapes = reference.is_empty()
        || Path::new(reference).is_absolute()
        || reference.split('/').any(|segment| segment.is_empty() || segment == "..");
    if escapes {
        bail!("invalid storage reference `{reference}`");
    }
    Ok(())
}

fn read_optional(path: &Path) -> Result<Option<Vec<u8>>> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err.into()),
    }
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    emery_artifacts::atomic::bytes_write(path, bytes)?;
    Ok(())
}

fn remove_if_present(path: &Path) -> Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err.into()),
    }
}

fn walk(root: &Path, dir: &Path, names: &mut Vec<String>) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            walk(root, &path, names)?;
        } else if let Ok(relative) = path.strip_prefix(root) {
            names.push(relative.to_string_lossy().replace(std::path::MAIN_SEPARATOR, "/"));
        }
    }
    Ok(())
}

fn unsupported<T: Send>(operation: &str) -> impl Future<Output = Result<T>> + Send + use<T> {
    ready(Err(anyhow::anyhow!("{operation} is unsupported by the filesystem storage backing")))
}

impl StateStore for Disk {
    fn get(&self, key: &str) -> impl Future<Output = Result<Option<Vec<u8>>>> + Send {
        ready(self.state_get(key))
    }

    fn set(
        &self, key: &str, value: &[u8], ttl_secs: Option<u64>,
    ) -> impl Future<Output = Result<Option<Vec<u8>>>> + Send {
        ready(self.state_set(key, value, ttl_secs))
    }

    fn delete(&self, key: &str) -> impl Future<Output = Result<()>> + Send {
        ready(self.state_delete(key))
    }

    fn cas(
        &self, key: &str, expected: Option<&[u8]>, value: &[u8],
    ) -> impl Future<Output = Result<(), CasError>> + Send {
        ready(self.state_cas(key, expected, value))
    }

    fn increment(&self, _key: &str, _delta: i64) -> impl Future<Output = Result<i64>> + Send {
        unsupported("increment")
    }
}

impl BlobStore for Disk {
    fn get(
        &self, container: &str, name: &str,
    ) -> impl Future<Output = Result<Option<Vec<u8>>>> + Send {
        ready(self.blob_get(container, name))
    }

    fn put(
        &self, container: &str, name: &str, data: &[u8],
    ) -> impl Future<Output = Result<()>> + Send {
        ready(self.blob_put(container, name, data))
    }

    fn delete(&self, container: &str, name: &str) -> impl Future<Output = Result<()>> + Send {
        ready(self.blob_delete(container, name))
    }

    fn has(&self, container: &str, name: &str) -> impl Future<Output = Result<bool>> + Send {
        ready(self.blob_has(container, name))
    }

    fn list(&self, container: &str) -> impl Future<Output = Result<Vec<String>>> + Send {
        ready(self.blob_list(container))
    }

    fn get_range(
        &self, _container: &str, _name: &str, _start: u64, _end: u64,
    ) -> impl Future<Output = Result<Vec<u8>>> + Send {
        unsupported("get_range")
    }

    fn object_info(
        &self, _container: &str, _name: &str,
    ) -> impl Future<Output = Result<ObjectMetadata>> + Send {
        unsupported("object_info")
    }

    fn delete_objects(
        &self, _container: &str, _names: &[String],
    ) -> impl Future<Output = Result<()>> + Send {
        unsupported("delete_objects")
    }

    fn clear(&self, _container: &str) -> impl Future<Output = Result<()>> + Send {
        unsupported("clear")
    }

    fn create_container(&self, _name: &str) -> impl Future<Output = Result<()>> + Send {
        unsupported("create_container")
    }

    fn delete_container(&self, _name: &str) -> impl Future<Output = Result<()>> + Send {
        unsupported("delete_container")
    }

    fn container_exists(&self, _name: &str) -> impl Future<Output = Result<bool>> + Send {
        unsupported("container_exists")
    }

    fn container_info(
        &self, _container: &str,
    ) -> impl Future<Output = Result<ContainerMetadata>> + Send {
        unsupported("container_info")
    }

    fn copy_object(
        &self, _src_container: &str, _src_name: &str, _dest_container: &str, _dest_name: &str,
    ) -> impl Future<Output = Result<()>> + Send {
        unsupported("copy_object")
    }

    fn move_object(
        &self, _src_container: &str, _src_name: &str, _dest_container: &str, _dest_name: &str,
    ) -> impl Future<Output = Result<()>> + Send {
        unsupported("move_object")
    }
}

// Map a storage capability failure onto the typed engine error: the
// stable `storage-failed` discriminant plus the acting context.
pub(crate) fn failed(action: &str, err: &anyhow::Error) -> emery_error::Error {
    emery_error::Error::Diag {
        code: "storage-failed",
        detail: format!("{action}: {err:#}"),
    }
}
