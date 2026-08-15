use crate::host::WasiExecCtxView;
use crate::host::generated::Error;
use crate::host::generated::emery::exec_mode::types::Host;

impl Host for WasiExecCtxView<'_> {
    fn convert_error(&mut self, err: Error) -> wasmtime::Result<Error> {
        Ok(err)
    }
}
