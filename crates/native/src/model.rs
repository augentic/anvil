//! [`DynModel`] — the native host's erased model backend.
//! `omnia_guest::Model` is not object-safe, so the composition root
//! erases it once; middleware composes before erasure.

use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use omnia_guest::Model;
use omnia_guest::model::{Error, Reply, Request};

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// The private object-safe erasure of [`Model`].
trait Erased: Send + Sync {
    fn create(&self, request: Request) -> BoxFuture<'_, Result<Reply, Error>>;
}

impl<M: Model> Erased for M {
    fn create(&self, request: Request) -> BoxFuture<'_, Result<Reply, Error>> {
        Box::pin(Model::create(self, request))
    }
}

/// A clone-cheap, type-erased [`Model`] backend.
#[derive(Clone)]
pub struct DynModel {
    inner: Arc<dyn Erased>,
}

impl DynModel {
    /// Erase `model` once at the composition root.
    ///
    /// Backends with post-run state (recording doubles, telemetry)
    /// expose it through caller-held clones taken before erasure —
    /// the provider exposes no model accessor.
    #[must_use]
    pub fn new(model: impl Model + 'static) -> Self {
        Self {
            inner: Arc::new(model),
        }
    }
}

impl fmt::Debug for DynModel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DynModel").finish_non_exhaustive()
    }
}

impl Model for DynModel {
    async fn create(&self, request: Request) -> Result<Reply, Error> {
        self.inner.create(request).await
    }
}
