//! Host side of the `emery:adapter/workspaces` capability (RFC-87).
//!
//! The engine guest's `workflow` world imports `workspaces` — freeze /
//! prepare / capture / discard plus the interim apply — and this crate
//! links a host implementation of that interface into the shipped
//! deployment, following the shared `omnia` host-crate shape
//! (`wasi-otel`, `wasi-model`). The capability is host-owned because
//! WASI exposes no unix mode bits: an in-guest capture would silently
//! lose the executable bit. The actual snapshot and materialization
//! kernel lives in `emery-project`'s `workspace` module; the
//! deployment backend (the launcher's `Workspaces`) binds it to the
//! invocation's captured layout through [`WasiWorkspacesCtx`].

mod generated {
    #![allow(
        missing_docs,
        clippy::pedantic,
        clippy::nursery,
        reason = "wasmtime bindgen generated bindings are not hand-maintained; the generated code cannot carry this workspace's lint posture"
    )]

    pub use self::emery::adapter::types::Error;

    wasmtime::component::bindgen!({
        world: "workspace-host",
        path: "../../wit",
        imports: {
            default: store | tracing | trappable,
        },
        trappable_error_type: {
            "emery:adapter/types.error" => Error,
        },
    });
}

use std::fmt::Debug;

pub use omnia::FutureResult;
use omnia::{Host, Server};
use wasmtime::component::{Accessor, HasData, Linker};

pub use self::generated::Error;
use self::generated::emery::adapter::workspaces;
pub use self::generated::emery::adapter::workspaces::{CodePatch, Prepared};

/// Host-side service for the workspaces capability (a linked-only
/// effect host).
#[derive(Clone, Copy, Debug)]
pub struct WasiWorkspaces;

impl HasData for WasiWorkspaces {
    type Data<'a> = WasiWorkspacesCtxView<'a>;
}

impl<T> Host<T> for WasiWorkspaces
where
    T: WasiWorkspacesView + 'static,
{
    fn add_to_linker(linker: &mut Linker<T>) -> anyhow::Result<()> {
        Ok(workspaces::add_to_linker::<_, Self>(linker, T::workspaces)?)
    }
}

impl<B> Server<B> for WasiWorkspaces {}

/// The backend trait — the one place the deployment's workspace policy
/// lives.
///
/// Values ride the WIT wire shapes: snapshot identities are the
/// canonical `sha256:<hex>` strings and workspace ids are the opaque
/// directory names beneath the deployment's workspaces mount.
pub trait WasiWorkspacesCtx: Debug + Send + Sync + 'static {
    /// Freeze the product tree as an immutable snapshot.
    fn freeze(&self) -> FutureResult<String>;

    /// Materialize `base` into a fresh private workspace
    /// (`writable: false` prepares a read-only source view).
    fn prepare(&self, base: String, writable: bool) -> FutureResult<Prepared>;

    /// Capture a workspace's result tree as a code patch.
    fn capture(&self, id: String) -> FutureResult<CodePatch>;

    /// Discard a workspace (idempotent).
    fn discard(&self, id: String) -> FutureResult<()>;

    /// Interim code delivery (pre-RFC-89): write `patch`'s touched
    /// paths onto the product tree.
    fn apply(&self, patch: CodePatch) -> FutureResult<()>;
}

/// Run one blocking workspace-kernel closure off the async executor —
/// the helper deployment backends wrap their filesystem-heavy legs in.
pub fn blocking<T: Send + 'static>(
    task: impl FnOnce() -> anyhow::Result<T> + Send + 'static,
) -> FutureResult<T> {
    use futures::FutureExt as _;
    async move { tokio::task::spawn_blocking(task).await? }.boxed()
}

impl<T> workspaces::HostWithStore<T> for WasiWorkspaces
where
    T: 'static,
{
    async fn freeze(accessor: &Accessor<T, Self>) -> Result<String, Error> {
        Ok(accessor.with(|mut store| store.get().ctx.freeze()).await?)
    }

    async fn prepare(
        accessor: &Accessor<T, Self>, base: String, writable: bool,
    ) -> Result<Prepared, Error> {
        Ok(accessor.with(|mut store| store.get().ctx.prepare(base, writable)).await?)
    }

    async fn capture(accessor: &Accessor<T, Self>, id: String) -> Result<CodePatch, Error> {
        Ok(accessor.with(|mut store| store.get().ctx.capture(id)).await?)
    }

    async fn discard(accessor: &Accessor<T, Self>, id: String) -> Result<(), Error> {
        Ok(accessor.with(|mut store| store.get().ctx.discard(id)).await?)
    }

    async fn apply(accessor: &Accessor<T, Self>, patch: CodePatch) -> Result<(), Error> {
        Ok(accessor.with(|mut store| store.get().ctx.apply(patch)).await?)
    }
}

impl workspaces::Host for WasiWorkspacesCtxView<'_> {}

impl generated::emery::adapter::types::Host for WasiWorkspacesCtxView<'_> {
    fn convert_error(&mut self, err: Error) -> wasmtime::Result<Error> {
        Ok(err)
    }
}

// An untyped host failure is an `internal` error at the boundary.
omnia::host_error!(Error, Internal);
omnia::wasi_view!(Workspaces);
