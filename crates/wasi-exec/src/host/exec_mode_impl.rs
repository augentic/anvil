use wasmtime::component::Accessor;

use crate::host::generated::Error;
use crate::host::generated::emery::exec_mode::exec_mode::{Host, HostWithStore};
use crate::host::{WasiExec, WasiExecCtxView};

impl<T> HostWithStore<T> for WasiExec
where
    T: 'static,
{
    async fn read(accessor: &Accessor<T, Self>, root: String) -> Result<Vec<String>, Error> {
        Ok(accessor.with(|mut store| store.get().ctx.read(root)).await?)
    }

    async fn apply(
        accessor: &Accessor<T, Self>, root: String, exec: Vec<String>, plain: Vec<String>,
    ) -> Result<(), Error> {
        Ok(accessor.with(|mut store| store.get().ctx.apply(root, exec, plain)).await?)
    }
}

impl Host for WasiExecCtxView<'_> {}
