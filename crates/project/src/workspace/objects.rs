//! Digest-named blob storage beneath the snapshot store.
//!
//! The kernel owns hashing; implementors stream bytes under that digest
//! so a large file need not fit in memory.

use std::fmt::Debug;
use std::future::Future;
use std::path::{Path, PathBuf};

use error::Error;

/// Digest-named object storage beneath the snapshot [`Store`](super::Store).
///
/// Implementors never hash: the kernel names every object by the
/// SHA-256 it computed and verifies content on read, so store
/// integrity does not depend on the backend. Operations are async to
/// match the deployment's `wasi:blobstore` import; each leg remains
/// quick local object I/O.
pub trait Objects: Debug + Send + Sync {
    /// Store `bytes` under `digest`. Write-once friendly: equal digest
    /// means equal content, so leaving an existing object untouched is
    /// correct.
    ///
    /// # Errors
    ///
    /// Storage failures.
    fn put(&self, digest: &str, bytes: &[u8]) -> impl Future<Output = Result<(), Error>> + Send;

    /// Stream the file at `src` into the object named `digest`.
    /// Write-once: an existing object is left untouched.
    ///
    /// # Errors
    ///
    /// Storage failures, including an unreadable `src`.
    fn put_file(&self, digest: &str, src: &Path) -> impl Future<Output = Result<(), Error>> + Send;

    /// Read the object named `digest`.
    ///
    /// # Errors
    ///
    /// Storage failures, including absence.
    fn get(&self, digest: &str) -> impl Future<Output = Result<Vec<u8>, Error>> + Send;

    /// Stream the object named `digest` into `dest`. The kernel
    /// verifies the written bytes; implementors do not hash.
    ///
    /// # Errors
    ///
    /// Storage failures, including absence.
    fn copy_file(
        &self, digest: &str, dest: &Path,
    ) -> impl Future<Output = Result<(), Error>> + Send;

    /// Whether the object named `digest` exists.
    fn has(&self, digest: &str) -> impl Future<Output = bool> + Send;

    /// Delete the object named `digest`. Idempotent — an absent
    /// object is already deleted.
    ///
    /// # Errors
    ///
    /// Storage failures other than absence.
    fn delete(&self, digest: &str) -> impl Future<Output = Result<(), Error>> + Send;
}

/// Filesystem objects sharded as `objects/<2 hex>/<62 hex>` beneath a
/// root directory.
///
/// Writes are atomic and write-once. Native test/lab deployments only
/// — the shipped deployment's store format is owned by its blobstore
/// backend.
#[derive(Clone, Debug)]
pub struct FsObjects {
    root: PathBuf,
}

impl FsObjects {
    /// Open the object tree at `root`; directories are created lazily
    /// on first write.
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// The object-tree root, for the store's self-exclusion walk.
    pub(crate) fn root(&self) -> &Path {
        &self.root
    }

    fn object_path(&self, digest: &str) -> PathBuf {
        self.root.join("objects").join(&digest[..2]).join(&digest[2..])
    }
}

impl Objects for FsObjects {
    async fn put(&self, digest: &str, bytes: &[u8]) -> Result<(), Error> {
        let path = self.object_path(digest);
        if !path.is_file() {
            artifacts::atomic::bytes_write(&path, bytes)?;
        }
        Ok(())
    }

    async fn put_file(&self, digest: &str, src: &Path) -> Result<(), Error> {
        let path = self.object_path(digest);
        if !path.is_file() {
            artifacts::atomic::copy_write(&path, src)?;
        }
        Ok(())
    }

    async fn get(&self, digest: &str) -> Result<Vec<u8>, Error> {
        let path = self.object_path(digest);
        std::fs::read(&path).map_err(|source| Error::Filesystem {
            op: "read",
            path,
            source,
        })
    }

    async fn copy_file(&self, digest: &str, dest: &Path) -> Result<(), Error> {
        let path = self.object_path(digest);
        std::fs::copy(&path, dest).map_err(|source| Error::Filesystem {
            op: "read",
            path,
            source,
        })?;
        Ok(())
    }

    async fn has(&self, digest: &str) -> bool {
        self.object_path(digest).is_file()
    }

    async fn delete(&self, digest: &str) -> Result<(), Error> {
        match std::fs::remove_file(self.object_path(digest)) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(Error::Io(err)),
        }
    }
}
