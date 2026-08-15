//! In-guest workspace-kernel seams: snapshot objects over Omnia's
//! `BlobStore` capability (the `wasi:blobstore` import), exec mode
//! over `emery:exec-mode`; tree I/O runs over the preopens.

use std::collections::BTreeSet;
use std::io::{Read, Write as _};
use std::path::Path;

use error::Error;
use omnia_guest::BlobStore;
use project::workspace::{ExecMode, Objects, Store};

use crate::bindings::emery::exec_mode::exec_mode;

/// The one container engine snapshots live in.
const CONTAINER: &str = "snapshots";

/// The in-guest snapshot store: blobstore-backed objects, host
/// exec mode.
///
/// # Errors
///
/// `snapshot-store-io` when the snapshots container cannot be opened.
pub(crate) async fn store() -> Result<Store<BlobObjects>, Error> {
    if !BlobObjects.container_exists(CONTAINER).await.map_err(store_error)? {
        BlobObjects.create_container(CONTAINER).await.map_err(store_error)?;
    }
    Ok(Store::over(BlobObjects, WitExecMode))
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

    async fn put_file(&self, digest: &str, src: &Path) -> Result<(), Error> {
        let name = object_name(digest);
        if BlobStore::has(self, CONTAINER, &name).await.unwrap_or(false) {
            return Ok(());
        }
        put_stream(&name, src).await
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

    async fn copy_file(&self, digest: &str, dest: &Path) -> Result<(), Error> {
        copy_stream(&object_name(digest), dest).await
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

/// [`ExecMode`] over the deployment's `emery:exec-mode` import —
/// `wasi:filesystem` carries no permission bits, so the host
/// round-trips the manifest's one mode distinction.
#[derive(Clone, Copy, Debug)]
struct WitExecMode;

impl ExecMode for WitExecMode {
    fn read(&self, root: &Path) -> Result<BTreeSet<String>, Error> {
        let paths = exec_mode::read(guest_root(root)?).map_err(exec_error)?;
        Ok(paths.into_iter().collect())
    }

    fn apply(&self, root: &Path, exec: &[String], plain: &[String]) -> Result<(), Error> {
        exec_mode::apply(guest_root(root)?, exec, plain).map_err(exec_error)
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

/// Stream `src` into blobstore object `name` without buffering the
/// file. `wasi:io` write budget is 4096 bytes per call.
async fn put_stream(name: &str, src: &Path) -> Result<(), Error> {
    use omnia_guest::anyhow::anyhow;
    use omnia_guest::omnia_wasi_blobstore::types::OutgoingValue;

    const WRITE_CHUNK: usize = 4096;

    let mut file = std::fs::File::open(src).map_err(|source| Error::Filesystem {
        op: "read",
        path: src.to_path_buf(),
        source,
    })?;
    let ctr = omnia_guest::omnia_wasi_blobstore::blobstore::get_container(CONTAINER.to_string())
        .await
        .map_err(|e| store_error(anyhow!("opening container: {e}")))?;
    let outgoing = OutgoingValue::new_outgoing_value();
    {
        let body = outgoing
            .outgoing_value_write_body()
            .await
            .map_err(|e| store_error(anyhow!("getting write body: {e:?}")))?;
        let mut buf = [0_u8; WRITE_CHUNK];
        loop {
            let n = file.read(&mut buf).map_err(|source| Error::Filesystem {
                op: "read",
                path: src.to_path_buf(),
                source,
            })?;
            if n == 0 {
                break;
            }
            body.blocking_write_and_flush(&buf[..n])
                .map_err(|e| store_error(anyhow!("writing data: {e}")))?;
        }
    }
    ctr.write_data(name.to_string(), &outgoing)
        .await
        .map_err(|e| store_error(anyhow!("writing object: {e}")))?;
    OutgoingValue::finish(outgoing).map_err(|e| store_error(anyhow!("finishing write: {e}")))
}

/// Stream blobstore object `name` into `dest` via ranged reads.
async fn copy_stream(name: &str, dest: &Path) -> Result<(), Error> {
    const RANGE_CHUNK: u64 = 64 * 1024;
    let objects = BlobObjects;
    let info = BlobStore::object_info(&objects, CONTAINER, name).await.map_err(store_error)?;
    let mut file = std::fs::File::create(dest).map_err(|source| Error::Filesystem {
        op: "write",
        path: dest.to_path_buf(),
        source,
    })?;
    if info.size == 0 {
        return Ok(());
    }
    let mut offset = 0_u64;
    while offset < info.size {
        let end = offset.saturating_add(RANGE_CHUNK - 1).min(info.size - 1);
        let chunk = BlobStore::get_range(&objects, CONTAINER, name, offset, end)
            .await
            .map_err(store_error)?;
        file.write_all(&chunk).map_err(|source| Error::Filesystem {
            op: "write",
            path: dest.to_path_buf(),
            source,
        })?;
        offset = end + 1;
    }
    Ok(())
}

fn store_error(error: omnia_guest::anyhow::Error) -> Error {
    Error::Diag {
        code: "snapshot-store-io",
        detail: format!("{error:#}"),
    }
}

fn exec_error(error: exec_mode::Error) -> Error {
    Error::Diag {
        code: "workspace-exec-mode",
        detail: match error {
            exec_mode::Error::InvalidRequest(detail)
            | exec_mode::Error::Io(detail)
            | exec_mode::Error::Internal(detail) => detail,
        },
    }
}
