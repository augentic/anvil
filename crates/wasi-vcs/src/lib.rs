//! Host side of the `emery:vcs` capability (RFC-95): Git and forge
//! I/O cannot run in the engine guest. The `trees`, `worktree`, and
//! `forge` interfaces are all wired.

mod generated {
    #![allow(
        missing_docs,
        clippy::pedantic,
        clippy::nursery,
        reason = "wasmtime bindgen generated bindings are not hand-maintained"
    )]

    pub use self::emery::vcs::forge::Error as ForgeError;
    pub use self::emery::vcs::trees::Error;
    pub use self::emery::vcs::worktree::ExportError;

    wasmtime::component::bindgen!({
        world: "vcs-host",
        path: "wit",
        imports: {
            default: store | tracing | trappable,
        },
        trappable_error_type: {
            "emery:vcs/trees.error" => Error,
            "emery:vcs/worktree.export-error" => ExportError,
            "emery:vcs/forge.error" => ForgeError,
        },
    });
}

use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;

use omnia::{Host, Server};
use wasmtime::component::{Accessor, HasData, Linker};

pub use self::generated::emery::vcs::forge::{PrState, PullRequest};
pub use self::generated::emery::vcs::trees::{Credentials, Fetched, Limits};
pub use self::generated::emery::vcs::worktree::{Request as ExportRequest, State as ExportState};
use self::generated::emery::vcs::{forge, trees, worktree};
pub use self::generated::{Error, ExportError, ForgeError};

/// A boxed backend future carrying the typed `trees.error` — the
/// backend's `limit` / `access` variants must survive to the guest,
/// so the anyhow-collapsing `omnia::FutureResult` does not fit here.
pub type TreesResult<T> = Pin<Box<dyn Future<Output = Result<T, Error>> + Send + 'static>>;

/// A boxed backend future carrying the typed `worktree.export-error` —
/// the D11 refusal rows must survive to the guest as typed variants.
pub type WorktreeResult<T> = Pin<Box<dyn Future<Output = Result<T, ExportError>> + Send + 'static>>;

/// A boxed backend future carrying the typed `forge.error` — auth and
/// transport must reach the guest as distinct outcomes (D10).
pub type ForgeResult<T> = Pin<Box<dyn Future<Output = Result<T, ForgeError>> + Send + 'static>>;

/// Host-side service for the VCS capability (a linked-only effect
/// host).
#[derive(Clone, Copy, Debug)]
pub struct WasiVcs;

impl HasData for WasiVcs {
    type Data<'a> = WasiVcsCtxView<'a>;
}

impl<T> Host<T> for WasiVcs
where
    T: WasiVcsView + 'static,
{
    fn add_to_linker(linker: &mut Linker<T>) -> anyhow::Result<()> {
        trees::add_to_linker::<_, Self>(linker, T::vcs)?;
        worktree::add_to_linker::<_, Self>(linker, T::vcs)?;
        Ok(forge::add_to_linker::<_, Self>(linker, T::vcs)?)
    }
}

impl<B> Server<B> for WasiVcs {}

/// The backend trait — the one place the deployment's Git transport lives.
///
/// Policy (CID minting, D9 metering, moved-branch checks) stays
/// engine-side; the backend stages trees and applies only the
/// transport-level limits it is handed.
pub trait WasiVcsCtx: Debug + Send + Sync + 'static {
    /// Stage `locator` beneath the deployment's staging root and
    /// report the resolved revision when the locator is Git.
    fn fetch(
        &self, locator: String, credentials: Credentials, limits: Limits,
    ) -> TreesResult<Fetched>;

    /// Discard a staged tree by its deployment-local root.
    /// Best-effort and idempotent.
    fn discard(&self, root: String) -> TreesResult<()>;

    /// One RFC-95 D11 publication materialize: provision the
    /// checkout, apply the closed state table, materialize the CID,
    /// stage the index. Returns the host worktree path and the
    /// idempotency state.
    fn export(&self, req: ExportRequest) -> WorktreeResult<(String, ExportState)>;

    /// Every open, merged, and closed pull request for
    /// `(repository, branch)`, pagination followed to exhaustion
    /// (RFC-95 D10). The trailer / digest / ordering checks are
    /// engine checks over these results.
    fn find(&self, repository: String, branch: String) -> ForgeResult<Vec<PullRequest>>;
}

/// Run one blocking fetch closure off the async executor — the helper
/// deployment backends wrap their git / HTTPS legs in.
pub fn blocking<T: Send + 'static>(
    task: impl FnOnce() -> Result<T, Error> + Send + 'static,
) -> TreesResult<T> {
    Box::pin(async move {
        tokio::task::spawn_blocking(task)
            .await
            .map_err(|join| Error::Internal(format!("vcs task panicked: {join}")))?
    })
}

/// Run one blocking forge closure off the async executor — the
/// helper deployment backends wrap their GitHub REST leg in.
pub fn blocking_forge<T: Send + 'static>(
    task: impl FnOnce() -> Result<T, ForgeError> + Send + 'static,
) -> ForgeResult<T> {
    Box::pin(async move {
        tokio::task::spawn_blocking(task)
            .await
            .map_err(|join| ForgeError::Internal(format!("forge task panicked: {join}")))?
    })
}

impl<T> trees::HostWithStore<T> for WasiVcs
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

impl trees::Host for WasiVcsCtxView<'_> {
    fn convert_error(&mut self, err: Error) -> wasmtime::Result<Error> {
        Ok(err)
    }
}

impl<T> worktree::HostWithStore<T> for WasiVcs
where
    T: 'static,
{
    async fn export(
        accessor: &Accessor<T, Self>, req: ExportRequest,
    ) -> Result<(String, ExportState), ExportError> {
        accessor.with(|mut store| store.get().ctx.export(req)).await
    }
}

impl worktree::Host for WasiVcsCtxView<'_> {
    fn convert_export_error(&mut self, err: ExportError) -> wasmtime::Result<ExportError> {
        Ok(err)
    }
}

impl<T> forge::HostWithStore<T> for WasiVcs
where
    T: 'static,
{
    async fn find(
        accessor: &Accessor<T, Self>, repository: String, branch: String,
    ) -> Result<Vec<PullRequest>, ForgeError> {
        accessor.with(|mut store| store.get().ctx.find(repository, branch)).await
    }
}

impl forge::Host for WasiVcsCtxView<'_> {
    fn convert_error(&mut self, err: ForgeError) -> wasmtime::Result<ForgeError> {
        Ok(err)
    }
}

// An untyped host failure is an `internal` error at the boundary.
omnia::host_error!(Error, Internal);
omnia::host_error!(ExportError, Internal);
omnia::host_error!(ForgeError, Internal);
omnia::wasi_view!(Vcs);
