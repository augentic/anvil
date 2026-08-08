//! Deployment backend for the workspaces host capability: the WIT
//! imports bound to the `project::workspace` kernel over the captured
//! layout. Every leg is filesystem-heavy, so each runs blocking.

use omnia::Backend;
use omnia_wasi_workspaces::{CodePatch, FutureResult, Prepared, WasiWorkspacesCtx, blocking};
use project::handler::ExecutionPaths;
use project::snapshot::SnapshotId;
use project::workspace::{self, Access, Store};

/// The workspaces backend: the engine's workspace kernel over this
/// invocation's captured layout.
#[derive(Clone, Debug)]
pub struct Workspaces {
    paths: ExecutionPaths,
}

impl Backend for Workspaces {
    type ConnectOptions = omnia::NoOptions;

    async fn connect_with(_options: omnia::NoOptions) -> anyhow::Result<Self> {
        Ok(Self {
            paths: super::current().paths.clone(),
        })
    }
}

impl WasiWorkspacesCtx for Workspaces {
    /// Freeze the product tree at the project root (the kernel
    /// excludes `.git` and `.emery`) — interim base pinning until
    /// RFC-86 records base pins.
    fn freeze(&self) -> FutureResult<String> {
        let paths = self.paths.clone();
        blocking(move || Ok(store(&paths).snapshot(paths.project_root())?.to_string()))
    }

    fn prepare(&self, base: String, writable: bool) -> FutureResult<Prepared> {
        let paths = self.paths.clone();
        blocking(move || {
            let base = SnapshotId::parse(&base)?;
            let root = paths.locations().workspaces_root().to_path_buf();
            let prepared = workspace::prepare(&store(&paths), &root, &base, Access { writable })?;
            // The artifact root is host-absolute so a spawned agent
            // whose working directory is the lent workspace can still
            // read change-tree artifacts.
            let artifacts = std::path::absolute(paths.project_root())
                .unwrap_or_else(|_io| paths.project_root().to_path_buf())
                .display()
                .to_string();
            Ok(Prepared {
                id: prepared.id,
                artifacts,
            })
        })
    }

    fn capture(&self, id: String) -> FutureResult<CodePatch> {
        let paths = self.paths.clone();
        blocking(move || {
            let root = paths.locations().workspaces_root().to_path_buf();
            let patch = workspace::capture(&store(&paths), &root, &id)?;
            Ok(CodePatch {
                base: patch.base.to_string(),
                result: patch.result.to_string(),
                touched: patch.touched,
            })
        })
    }

    fn discard(&self, id: String) -> FutureResult<()> {
        let paths = self.paths.clone();
        blocking(move || {
            let root = paths.locations().workspaces_root().to_path_buf();
            Ok(workspace::discard(&root, &id)?)
        })
    }

    /// Interim code delivery (pre-RFC-89): write `patch`'s touched
    /// paths onto the product tree at the project root.
    fn apply(&self, patch: CodePatch) -> FutureResult<()> {
        let paths = self.paths.clone();
        blocking(move || {
            let patch = project::snapshot::CodePatch {
                base: SnapshotId::parse(&patch.base)?,
                result: SnapshotId::parse(&patch.result)?,
                touched: patch.touched,
            };
            Ok(store(&paths).apply(&patch, paths.project_root())?)
        })
    }
}

/// The snapshot store at the captured locations' snapshots root.
fn store(paths: &ExecutionPaths) -> Store {
    Store::new(paths.locations().snapshots_root())
}
