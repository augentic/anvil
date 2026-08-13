//! Host side of the `emery:ingest` capability. Git clone and HTTPS
//! fetch cannot run in the engine guest.

mod generated {
    #![allow(
        missing_docs,
        clippy::pedantic,
        clippy::nursery,
        reason = "wasmtime bindgen generated bindings are not hand-maintained"
    )]

    pub use self::emery::ingest::types::Error;

    wasmtime::component::bindgen!({
        world: "ingest-host",
        path: "wit",
        imports: {
            default: store | tracing | trappable,
        },
        trappable_error_type: {
            "emery:ingest/types.error" => Error,
        },
    });
}

use std::fmt::Debug;

pub use omnia::FutureResult;
use omnia::{Host, Server};
use wasmtime::component::{Accessor, HasData, Linker};

pub use self::generated::Error;
use self::generated::emery::ingest::ingest;
pub use self::generated::emery::ingest::types::Fetched;

/// Host-side service for the ingest capability (a linked-only
/// effect host).
#[derive(Clone, Copy, Debug)]
pub struct WasiIngest;

impl HasData for WasiIngest {
    type Data<'a> = WasiIngestCtxView<'a>;
}

impl<T> Host<T> for WasiIngest
where
    T: WasiIngestView + 'static,
{
    fn add_to_linker(linker: &mut Linker<T>) -> anyhow::Result<()> {
        Ok(ingest::add_to_linker::<_, Self>(linker, T::ingest)?)
    }
}

impl<B> Server<B> for WasiIngest {}

/// The backend trait — Git/HTTPS staging plus CID snapshot.
pub trait WasiIngestCtx: Debug + Send + Sync + 'static {
    /// Stage `locator` and return the exact pin, CID, and tree path.
    fn fetch(
        &self, locator: String, recorded: Option<String>, prior: Option<String>,
    ) -> FutureResult<Fetched>;
}

/// Run one blocking/async closure off the executor.
pub fn blocking<T: Send + 'static>(
    task: impl FnOnce() -> anyhow::Result<T> + Send + 'static,
) -> FutureResult<T> {
    use futures::FutureExt as _;
    async move { tokio::task::spawn_blocking(task).await? }.boxed()
}

impl<T> ingest::HostWithStore<T> for WasiIngest
where
    T: 'static,
{
    async fn fetch(
        accessor: &Accessor<T, Self>, locator: String, recorded: Option<String>,
        prior: Option<String>,
    ) -> Result<Fetched, Error> {
        Ok(accessor.with(|mut store| store.get().ctx.fetch(locator, recorded, prior)).await?)
    }
}

impl ingest::Host for WasiIngestCtxView<'_> {}

impl generated::emery::ingest::types::Host for WasiIngestCtxView<'_> {
    fn convert_error(&mut self, err: Error) -> wasmtime::Result<Error> {
        Ok(err)
    }
}

omnia::host_error!(Error, Internal);
omnia::wasi_view!(Ingest);
