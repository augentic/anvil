//! In-guest workspace-kernel seams: snapshot objects over Omnia's
//! `BlobStore` capability (the `wasi:blobstore` import), exec bits
//! over `emery:exec-bits`; tree I/O runs over the preopens.

use std::collections::BTreeSet;
use std::path::Path;

use error::Error;
use omnia_guest::BlobStore;
use project::workspace::{ExecBits, Objects, Store};

use crate::bindings::emery::exec_bits::exec_bits;

/// The one container engine snapshots live in.
const CONTAINER: &str = "snapshots";

/// The in-guest snapshot store: blobstore-backed objects, host
/// exec bits.
///
/// # Errors
///
/// `snapshot-store-io` when the snapshots container cannot be opened.
pub(crate) async fn store() -> Result<Store<BlobObjects>, Error> {
    if !BlobObjects.container_exists(CONTAINER).await.map_err(store_error)? {
        BlobObjects.create_container(CONTAINER).await.map_err(store_error)?;
    }
    Ok(Store::over(BlobObjects, WitExecBits))
}

/// Digest-named objects sharded as `<2 hex>/<62 hex>` — an emery-owned
/// convention (shared with the future native verifier); the generic
/// backend maps `/` to a subdirectory.
fn object_name(digest: &str) -> String {
    format!("{}/{}", &digest[..2], &digest[2..])
}

/// [`Objects`] over Omnia's [`BlobStore`] capability, whose wasm32
/// default bodies drive the deployment's `wasi:blobstore` import.
#[derive(Debug)]
pub(crate) struct BlobObjects;

impl BlobStore for BlobObjects {}

impl Objects for BlobObjects {
    async fn put(&self, digest: &str, bytes: &[u8]) -> Result<(), Error> {
        let name = object_name(digest);
        // Write-once: equal digest means equal content, so an existing
        // object is left untouched.
        if BlobStore::has(self, CONTAINER, &name).await.unwrap_or(false) {
            return Ok(());
        }
        BlobStore::put(self, CONTAINER, &name, bytes).await.map_err(store_error)
    }

    async fn get(&self, digest: &str) -> Result<Vec<u8>, Error> {
        BlobStore::get(self, CONTAINER, &object_name(digest))
            .await
            .map_err(store_error)?
            .ok_or_else(|| Error::Diag {
                code: "snapshot-store-io",
                detail: format!("object `{digest}` is not in the store"),
            })
    }

    async fn has(&self, digest: &str) -> bool {
        BlobStore::has(self, CONTAINER, &object_name(digest)).await.unwrap_or(false)
    }

    async fn delete(&self, digest: &str) -> Result<(), Error> {
        let name = object_name(digest);
        // Idempotent: an absent object is already deleted.
        if !BlobStore::has(self, CONTAINER, &name).await.unwrap_or(false) {
            return Ok(());
        }
        BlobStore::delete(self, CONTAINER, &name).await.map_err(store_error)
    }
}

/// [`ExecBits`] over the deployment's `emery:exec-bits` import —
/// `wasi:filesystem` carries no permission bits, so the host
/// round-trips the manifest's one mode distinction.
#[derive(Clone, Copy, Debug)]
struct WitExecBits;

impl ExecBits for WitExecBits {
    fn read(&self, root: &Path) -> Result<BTreeSet<String>, Error> {
        let paths = exec_bits::read(guest_root(root)?).map_err(exec_error)?;
        Ok(paths.into_iter().collect())
    }

    fn apply(&self, root: &Path, exec: &[String], plain: &[String]) -> Result<(), Error> {
        exec_bits::apply(guest_root(root)?, exec, plain).map_err(exec_error)
    }
}

/// The deployment-local tree root as the wire string (`.` or a
/// workspace root beneath the workspaces mount).
fn guest_root(root: &Path) -> Result<&str, Error> {
    root.to_str().ok_or_else(|| Error::Diag {
        code: "workspace-path-unsupported",
        detail: format!("root `{}` is not UTF-8", root.display()),
    })
}

fn store_error(error: omnia_guest::anyhow::Error) -> Error {
    Error::Diag {
        code: "snapshot-store-io",
        detail: format!("{error:#}"),
    }
}

fn exec_error(error: exec_bits::Error) -> Error {
    Error::Diag {
        code: "workspace-exec-bits",
        detail: match error {
            exec_bits::Error::InvalidRequest(detail)
            | exec_bits::Error::Io(detail)
            | exec_bits::Error::Internal(detail) => detail,
        },
    }
}
