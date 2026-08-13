//! Deployment backend for the origins host capability: the kernel's
//! Git / HTTPS fetch into `origin-<nonce>` trees beneath the
//! workspaces root, reported under the workspaces mount.

use anyhow::bail;
use omnia::Backend;
use omnia_wasi_origins::{Fetched, FutureResult, WasiOriginsCtx, blocking};
use project::handler::{ExecutionPaths, GUEST_WORKSPACES_MOUNT};

/// The origins backend over this invocation's captured layout.
#[derive(Clone, Debug)]
pub struct Origins {
    paths: ExecutionPaths,
}

impl Backend for Origins {
    type ConnectOptions = omnia::NoOptions;

    async fn connect_with(_options: omnia::NoOptions) -> anyhow::Result<Self> {
        Ok(Self {
            paths: super::current().paths.clone(),
        })
    }
}

impl WasiOriginsCtx for Origins {
    fn fetch(&self, locator: String) -> FutureResult<Fetched> {
        let paths = self.paths.clone();
        blocking(move || {
            let parent = paths.locations().workspaces_root();
            let fetched = project::origins::fetch(parent, &locator)?;
            Ok(Fetched {
                root: format!("{GUEST_WORKSPACES_MOUNT}/{}", fetched.name),
                revision: fetched.revision,
            })
        })
    }

    fn discard(&self, root: String) -> FutureResult<()> {
        let paths = self.paths.clone();
        blocking(move || {
            let name = fetched_name(&root)?;
            Ok(project::origins::discard(paths.locations().workspaces_root(), &name)?)
        })
    }
}

/// The fetch-tree name inside a guest-reported root — refuses
/// anything not directly beneath the workspaces mount.
fn fetched_name(root: &str) -> anyhow::Result<String> {
    if let Some(name) =
        root.strip_prefix(GUEST_WORKSPACES_MOUNT).and_then(|rest| rest.strip_prefix('/'))
        && !name.is_empty()
        && !name.contains('/')
    {
        return Ok(name.to_string());
    }
    bail!("origins root `{root}` is not beneath the workspaces mount")
}
