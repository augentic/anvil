//! Deployment backend for the exec-mode host capability: guest-visible
//! roots validated onto the captured layout, mode bits round-tripped
//! by the kernel's native exec seam. Filesystem-heavy legs run blocking.

use std::path::PathBuf;

use anyhow::bail;
use omnia::Backend;
use omnia_wasi_execmode::{FutureResult, WasiExecModeCtx, blocking};
use project::handler::{ExecutionPaths, GUEST_WORKSPACES_MOUNT};
use project::workspace::{ExecMode as _, FsExecMode};

/// The exec-mode backend over this invocation's captured layout.
#[derive(Clone, Debug)]
pub struct ExecMode {
    paths: ExecutionPaths,
}

impl Backend for ExecMode {
    type ConnectOptions = omnia::NoOptions;

    async fn connect_with(_options: omnia::NoOptions) -> anyhow::Result<Self> {
        Ok(Self {
            paths: super::current().paths.clone(),
        })
    }
}

impl WasiExecModeCtx for ExecMode {
    fn read(&self, root: String) -> FutureResult<Vec<String>> {
        let paths = self.paths.clone();
        blocking(move || {
            let dir = resolve_root(&paths, &root)?;
            Ok(FsExecMode.read(&dir)?.into_iter().collect())
        })
    }

    fn apply(&self, root: String, exec: Vec<String>, plain: Vec<String>) -> FutureResult<()> {
        let paths = self.paths.clone();
        blocking(move || {
            let dir = resolve_root(&paths, &root)?;
            for path in exec.iter().chain(plain.iter()) {
                check_rel(path)?;
            }
            Ok(FsExecMode.apply(&dir, &exec, &plain)?)
        })
    }
}

/// Map a guest-visible root onto the captured layout: `.` is the
/// project tree, `<workspaces mount>/<id>` a private workspace.
/// Everything else — absolute paths, traversal, unknown prefixes —
/// refuses, the same discipline as the kernel's workspace-id check.
fn resolve_root(paths: &ExecutionPaths, root: &str) -> anyhow::Result<PathBuf> {
    if root == "." {
        return Ok(paths.project_root().to_path_buf());
    }
    if let Some(id) = root.strip_prefix(GUEST_WORKSPACES_MOUNT)
        && let Some(id) = id.strip_prefix('/')
        && !id.is_empty()
        && id.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return Ok(paths.locations().workspaces_root().join(id));
    }
    bail!("exec-mode root `{root}` is not a deployment-local tree root")
}

/// Refuse anything but a plain `/`-separated relative path.
fn check_rel(path: &str) -> anyhow::Result<()> {
    let plain = !path.is_empty()
        && !path.starts_with('/')
        && path.split('/').all(|segment| !segment.is_empty() && segment != "." && segment != "..");
    if plain { Ok(()) } else { bail!("exec-mode path `{path}` is not a tree-relative path") }
}
