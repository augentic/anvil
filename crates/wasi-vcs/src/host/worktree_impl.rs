use wasmtime::component::Accessor;

use crate::host::generated::emery::vcs::worktree::{
    ExportError, Host, HostWithStore, Request as ExportRequest, State as ExportState,
};
use crate::host::{WasiVcs, WasiVcsCtxView};

impl<T> HostWithStore<T> for WasiVcs
where
    T: 'static,
{
    async fn export(
        accessor: &Accessor<T, Self>, req: ExportRequest,
    ) -> Result<(String, ExportState), ExportError> {
        accessor.with(|mut store| store.get().ctx.export(req)).await
    }
}

impl Host for WasiVcsCtxView<'_> {
    fn convert_export_error(&mut self, err: ExportError) -> wasmtime::Result<ExportError> {
        Ok(err)
    }
}
