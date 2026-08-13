//! Host side of the `emery:origins` capability: the engine guest has
//! no network or git, so remote coverage locators (Git / HTTPS)
//! fetch host-side into a deployment-local tree.

mod generated {
    #![allow(
        missing_docs,
        clippy::pedantic,
        clippy::nursery,
        reason = "wasmtime bindgen generated bindings are not hand-maintained"
    )]

    pub use self::emery::origins::types::Error;

    wasmtime::component::bindgen!({
        world: "origins-host",
        path: "wit",
        imports: {
            default: store | tracing | trappable,
        },
        trappable_error_type: {
            "emery:origins/types.error" => Error,
        },
    });
}

use std::fmt::Debug;

pub use omnia::FutureResult;
use omnia::{Host, Server};
use wasmtime::component::{Accessor, HasData, Linker};

pub use self::generated::Error;
use self::generated::emery::origins::origins;
pub use self::generated::emery::origins::origins::Fetched;

/// Host-side service for the origins capability (a linked-only
/// effect host).
#[derive(Clone, Copy, Debug)]
pub struct WasiOrigins;

impl HasData for WasiOrigins {
    type Data<'a> = WasiOriginsCtxView<'a>;
}

impl<T> Host<T> for WasiOrigins
where
    T: WasiOriginsView + 'static,
{
    fn add_to_linker(linker: &mut Linker<T>) -> anyhow::Result<()> {
        Ok(origins::add_to_linker::<_, Self>(linker, T::origins)?)
    }
}

impl<B> Server<B> for WasiOrigins {}

/// The backend trait — the one place the deployment's fetch policy
/// lives.
///
/// `fetch` resolves a remote locator into a deployment-local tree
/// beneath the workspaces mount and reports the origin's revision
/// when it is Git; `discard` removes a fetched tree by that same
/// deployment-local root.
pub trait WasiOriginsCtx: Debug + Send + Sync + 'static {
    /// Fetch `locator` into a deployment-local tree.
    fn fetch(&self, locator: String) -> FutureResult<Fetched>;

    /// Discard a fetched tree by its deployment-local root.
    fn discard(&self, root: String) -> FutureResult<()>;
}

/// Run one blocking fetch closure off the async executor — the helper
/// deployment backends wrap their git / HTTPS legs in.
pub fn blocking<T: Send + 'static>(
    task: impl FnOnce() -> anyhow::Result<T> + Send + 'static,
) -> FutureResult<T> {
    use futures::FutureExt as _;
    async move { tokio::task::spawn_blocking(task).await? }.boxed()
}

impl<T> origins::HostWithStore<T> for WasiOrigins
where
    T: 'static,
{
    async fn fetch(accessor: &Accessor<T, Self>, locator: String) -> Result<Fetched, Error> {
        Ok(accessor.with(|mut store| store.get().ctx.fetch(locator)).await?)
    }

    async fn discard(accessor: &Accessor<T, Self>, root: String) -> Result<(), Error> {
        Ok(accessor.with(|mut store| store.get().ctx.discard(root)).await?)
    }
}

impl origins::Host for WasiOriginsCtxView<'_> {}

impl generated::emery::origins::types::Host for WasiOriginsCtxView<'_> {
    fn convert_error(&mut self, err: Error) -> wasmtime::Result<Error> {
        Ok(err)
    }
}

// An untyped host failure is an `internal` error at the boundary.
omnia::host_error!(Error, Internal);
omnia::wasi_view!(Origins);
