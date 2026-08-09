//! In-guest workspace-kernel seams: snapshot objects over the
//! `wasi:blobstore` import, exec bits over `emery:exec-bits`; tree
//! I/O runs over the `.` and workspaces preopens in the kernel.

/// Bindings over the vendored `wasi:blobstore` package (see
/// `wit/README.md`). Sync-lowered: the workspace kernel is
/// synchronous, and every blobstore leg is quick local object I/O.
mod blob_bindings {
    #![allow(missing_docs)]

    wit_bindgen::generate!({
        world: "snapshots",
        path: "wit",
        generate_all,
        async: false,
    });
}

use std::collections::BTreeSet;
use std::path::Path;

use error::Error;
use project::workspace::{ExecBits, Objects, Store};

use self::blob_bindings::wasi::blobstore::blobstore as blob;
use self::blob_bindings::wasi::blobstore::container::Container;
use self::blob_bindings::wasi::blobstore::types::{IncomingValue, OutgoingValue};
use crate::bindings::emery::exec_bits::exec_bits;

/// The one container engine snapshots live in.
const CONTAINER: &str = "snapshots";

/// `blocking-write-and-flush` accepts at most 4096 bytes per call.
const WRITE_CHUNK: usize = 4096;

/// The in-guest snapshot store: blobstore-backed objects, host
/// exec bits.
///
/// # Errors
///
/// `snapshot-store-io` when the snapshots container cannot be opened.
pub(crate) fn store() -> Result<Store, Error> {
    Ok(Store::over(BlobObjects::open()?, WitExecBits))
}

/// Digest-named objects sharded as `<2 hex>/<62 hex>` — an emery-owned
/// convention (shared with the future native verifier); the generic
/// backend maps `/` to a subdirectory.
fn object_name(digest: &str) -> String {
    format!("{}/{}", &digest[..2], &digest[2..])
}

/// [`Objects`] over the deployment's `wasi:blobstore` import.
#[derive(Debug)]
struct BlobObjects {
    container: Container,
}

impl BlobObjects {
    fn open() -> Result<Self, Error> {
        let exists = blob::container_exists(CONTAINER).map_err(store_error)?;
        let container =
            if exists { blob::get_container(CONTAINER) } else { blob::create_container(CONTAINER) }
                .map_err(store_error)?;
        Ok(Self { container })
    }
}

impl Objects for BlobObjects {
    fn put(&self, digest: &str, bytes: &[u8]) -> Result<(), Error> {
        let name = object_name(digest);
        if self.container.has_object(&name).map_err(store_error)? {
            return Ok(());
        }
        let value = OutgoingValue::new_outgoing_value();
        {
            // The stream is a child resource: it must drop before the
            // host consumes the buffered value below.
            let body = value
                .outgoing_value_write_body()
                .map_err(|()| store_error("outgoing-value body already taken".to_string()))?;
            for chunk in bytes.chunks(WRITE_CHUNK) {
                body.blocking_write_and_flush(chunk)
                    .map_err(|err| store_error(format!("{err:?}")))?;
            }
        }
        // `write-data` reads the buffered bytes; `finish` marks the
        // value complete rather than flushing it.
        self.container.write_data(&name, &value).map_err(store_error)?;
        OutgoingValue::finish(value).map_err(store_error)?;
        Ok(())
    }

    fn get(&self, digest: &str) -> Result<Vec<u8>, Error> {
        let value =
            self.container.get_data(&object_name(digest), 0, u64::MAX).map_err(store_error)?;
        IncomingValue::incoming_value_consume_sync(value).map_err(store_error)
    }

    fn has(&self, digest: &str) -> bool {
        self.container.has_object(&object_name(digest)).unwrap_or(false)
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

fn store_error(detail: String) -> Error {
    Error::Diag {
        code: "snapshot-store-io",
        detail,
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
