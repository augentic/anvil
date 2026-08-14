//! Host side of the `emery:exec-bits` capability. Exists solely
//! because `wasi:filesystem` carries no permission bits (still true
//! at 0.3.0); deletes if upstream bits ever land in Omnia's host.

mod generated {
    #![allow(
        missing_docs,
        clippy::pedantic,
        clippy::nursery,
        reason = "wasmtime bindgen generated bindings are not hand-maintained"
    )]

    pub use self::emery::exec_bits::types::Error;

    wasmtime::component::bindgen!({
        world: "exec-bits-host",
        path: "wit",
        imports: {
            default: store | tracing | trappable,
        },
        trappable_error_type: {
            "emery:exec-bits/types.error" => Error,
        },
    });
}

use std::fmt::Debug;

pub use omnia::FutureResult;
use omnia::{Host, Server};
use wasmtime::component::{Accessor, HasData, Linker};

pub use self::generated::Error;
use self::generated::emery::exec_bits::exec_bits;

/// Host-side service for the exec-bits capability (a linked-only
/// effect host).
#[derive(Clone, Copy, Debug)]
pub struct WasiExecBits;

impl HasData for WasiExecBits {
    type Data<'a> = WasiExecBitsCtxView<'a>;
}

impl<T> Host<T> for WasiExecBits
where
    T: WasiExecBitsView + 'static,
{
    fn add_to_linker(linker: &mut Linker<T>) -> anyhow::Result<()> {
        Ok(exec_bits::add_to_linker::<_, Self>(linker, T::execbits)?)
    }
}

impl<B> Server<B> for WasiExecBits {}

/// The backend trait — the one place the deployment's root-mapping
/// policy lives.
///
/// Roots ride the WIT wire shape: deployment-local tree roots (`.` or
/// a workspace root beneath the deployment's workspaces mount); paths
/// are `/`-separated and relative to the root.
pub trait WasiExecBitsCtx: Debug + Send + Sync + 'static {
    /// The relative paths of executable regular files beneath `root`.
    fn read(&self, root: String) -> FutureResult<Vec<String>>;

    /// Set the executable bit on `exec` and clear it on `plain`.
    fn apply(&self, root: String, exec: Vec<String>, plain: Vec<String>) -> FutureResult<()>;
}

/// Run one blocking filesystem closure off the async executor — the
/// helper deployment backends wrap their chmod/stat legs in.
pub fn blocking<T: Send + 'static>(
    task: impl FnOnce() -> anyhow::Result<T> + Send + 'static,
) -> FutureResult<T> {
    use futures::FutureExt as _;
    async move { tokio::task::spawn_blocking(task).await? }.boxed()
}

impl<T> exec_bits::HostWithStore<T> for WasiExecBits
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

impl exec_bits::Host for WasiExecBitsCtxView<'_> {}

impl generated::emery::exec_bits::types::Host for WasiExecBitsCtxView<'_> {
    fn convert_error(&mut self, err: Error) -> wasmtime::Result<Error> {
        Ok(err)
    }
}

// An untyped host failure is an `internal` error at the boundary.
omnia::host_error!(Error, Internal);
omnia::wasi_view!(ExecBits);
