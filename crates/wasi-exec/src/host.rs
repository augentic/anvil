//! Host-side service for `emery:exec-mode`.

mod default_impl;
mod exec_mode_impl;
mod types_impl;

mod generated {
    #![allow(
        missing_docs,
        clippy::pedantic,
        clippy::nursery,
        reason = "wasmtime bindgen generated bindings are not hand-maintained"
    )]

    pub use self::emery::exec_mode::types::Error;

    wasmtime::component::bindgen!({
        world: "imports",
        path: "wit",
        imports: {
            default: store | tracing | trappable,
        },
        trappable_error_type: {
            "emery:exec-mode/types.error" => Error,
        },
    });
}

use std::fmt::Debug;

pub use omnia::FutureResult;
use omnia::{Host, Server};
use wasmtime::component::{HasData, Linker};

pub use self::default_impl::ExecDefault;
pub use self::generated::Error;
use self::generated::emery::exec_mode::exec_mode;

/// Host-side service for `emery:exec-mode`.
#[derive(Clone, Copy, Debug)]
pub struct WasiExec;

impl HasData for WasiExec {
    type Data<'a> = WasiExecCtxView<'a>;
}

impl<T> Host<T> for WasiExec
where
    T: WasiExecView + 'static,
{
    fn add_to_linker(linker: &mut Linker<T>) -> anyhow::Result<()> {
        Ok(exec_mode::add_to_linker::<_, Self>(linker, T::exec)?)
    }
}

impl<B> Server<B> for WasiExec {}

/// A trait which provides internal exec-mode context.
///
/// Roots ride the WIT wire shape: deployment-local tree roots (`.` or
/// a workspace root beneath the deployment's workspaces mount); paths
/// are `/`-separated and relative to the root.
pub trait WasiExecCtx: Debug + Send + Sync + 'static {
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

omnia::host_error!(Error, Internal);
omnia::wasi_view!(Exec);
