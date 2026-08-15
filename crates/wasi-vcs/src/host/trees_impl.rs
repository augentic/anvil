use wasmtime::component::Accessor;

use crate::host::generated::emery::vcs::trees::{
    Credentials, Error, Fetched, Host, HostWithStore, Limits,
};
use crate::host::{WasiVcs, WasiVcsCtxView};

impl<T> HostWithStore<T> for WasiVcs
where
    T: 'static,
{
    async fn fetch(
        accessor: &Accessor<T, Self>, locator: String, credentials: Credentials, limits: Limits,
    ) -> Result<Fetched, Error> {
        accessor.with(|mut store| store.get().ctx.fetch(locator, credentials, limits)).await
    }

    async fn discard(accessor: &Accessor<T, Self>, root: String) -> Result<(), Error> {
        accessor.with(|mut store| store.get().ctx.discard(root)).await
    }
}

impl Host for WasiVcsCtxView<'_> {
    fn convert_error(&mut self, err: Error) -> wasmtime::Result<Error> {
        Ok(err)
    }
}
