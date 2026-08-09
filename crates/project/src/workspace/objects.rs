//! The object seam beneath the snapshot store: digest-named blobs.
//!
//! The kernel owns hashing and verify-on-read; an implementor only
//! stores and returns bytes under the digest the kernel computed.

use std::fmt::Debug;
use std::path::{Path, PathBuf};

use error::Error;

/// Digest-named object storage beneath the snapshot [`Store`](super::Store).
///
/// Implementors never hash: the kernel names every object by the
/// SHA-256 it computed and verifies content on read, so store
/// integrity does not depend on the backend.
pub trait Objects: Debug + Send + Sync {
    /// Store `bytes` under `digest`. Write-once friendly: equal digest
    /// means equal content, so leaving an existing object untouched is
    /// correct.
    ///
    /// # Errors
    ///
    /// Storage failures.
    fn put(&self, digest: &str, bytes: &[u8]) -> Result<(), Error>;

    /// Read the object named `digest`.
    ///
    /// # Errors
    ///
    /// Storage failures, including absence.
    fn get(&self, digest: &str) -> Result<Vec<u8>, Error>;

    /// Whether the object named `digest` exists.
    fn has(&self, digest: &str) -> bool;
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
    fn put(&self, digest: &str, bytes: &[u8]) -> Result<(), Error> {
        let path = self.object_path(digest);
        if !path.is_file() {
            artifacts::atomic::bytes_write(&path, bytes)?;
        }
        Ok(())
    }

    fn get(&self, digest: &str) -> Result<Vec<u8>, Error> {
        let path = self.object_path(digest);
        std::fs::read(&path).map_err(|source| Error::Filesystem {
            op: "read",
            path,
            source,
        })
    }

    fn has(&self, digest: &str) -> bool {
        self.object_path(digest).is_file()
    }
}
