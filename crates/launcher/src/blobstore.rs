//! Deployment backend for the blobstore host capability: the generic
//! filesystem blobstore anchored at the captured locations' snapshots
//! root, constructed programmatically (no environment reads).

use std::sync::Arc;

use omnia::Backend;
use omnia_wasi_blobstore::{Container, FutureResult, WasiBlobstoreCtx};

/// The blobstore backend: the filesystem object store at this
/// invocation's snapshots root.
///
/// The generic backend carries no emery policy — only the trusted
/// engine guest's world reaches `wasi:blobstore`, and the root is
/// scoped to the snapshots directory. If narrowing is later wanted
/// (single `snapshots` container, refuse `delete-container`), it is a
/// thin gate in this newtype.
#[derive(Clone, Debug)]
pub struct Blobstore {
    inner: omnia_filesystem::Client,
}

impl Backend for Blobstore {
    type ConnectOptions = omnia::NoOptions;

    async fn connect_with(_options: omnia::NoOptions) -> anyhow::Result<Self> {
        let root = super::current().paths.locations().snapshots_root().to_path_buf();
        Ok(Self {
            inner: omnia_filesystem::Client::open(root)?,
        })
    }
}

impl WasiBlobstoreCtx for Blobstore {
    fn create_container(&self, name: String) -> FutureResult<Arc<dyn Container>> {
        self.inner.create_container(name)
    }

    fn get_container(&self, name: String) -> FutureResult<Arc<dyn Container>> {
        self.inner.get_container(name)
    }

    fn delete_container(&self, name: String) -> FutureResult<()> {
        self.inner.delete_container(name)
    }

    fn container_exists(&self, name: String) -> FutureResult<bool> {
        self.inner.container_exists(name)
    }
}
