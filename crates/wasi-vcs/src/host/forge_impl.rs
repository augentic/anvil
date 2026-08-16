use wasmtime::component::Accessor;

use crate::host::generated::emery::vcs::forge::{
    Error as ForgeError, Host, HostWithStore, PullRequest,
};
use crate::host::{WasiVcs, WasiVcsCtxView};

impl<T> HostWithStore<T> for WasiVcs
where
    T: 'static,
{
    async fn find(
        accessor: &Accessor<T, Self>, repository: String, branch: String,
    ) -> Result<Vec<PullRequest>, ForgeError> {
        accessor.with(|mut store| store.get().ctx.find(repository, branch)).await
    }
}

impl Host for WasiVcsCtxView<'_> {
    fn convert_error(&mut self, err: ForgeError) -> wasmtime::Result<ForgeError> {
        Ok(err)
    }
}
