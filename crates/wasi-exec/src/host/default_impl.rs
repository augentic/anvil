//! Default implementation for `emery:exec-mode`.

use std::path::{Path, PathBuf};

use anyhow::bail;
use omnia::Backend;
use project::handler::{ExecutionPaths, GUEST_STAGING_MOUNT, GUEST_WORKSPACES_MOUNT};
use project::workspace::{ExecMode as _, FsExecMode};

use crate::host::{FutureResult, WasiExecCtx, blocking};

/// Default implementation for `emery:exec-mode`.
#[derive(Clone, Debug)]
pub struct ExecDefault {
    paths: ExecutionPaths,
}

impl Backend for ExecDefault {
    type ConnectOptions = omnia::NoOptions;

    async fn connect_with(_options: omnia::NoOptions) -> anyhow::Result<Self> {
        Ok(Self {
            paths: ExecutionPaths::host(),
        })
    }
}

impl WasiExecCtx for ExecDefault {
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
/// project tree; a path beneath the workspaces mount is a private
/// workspace, an ingest scratch tree, or a selected subtree; a path
/// beneath the staging mount is a staged VCS fetch (or a selected
/// subtree of one) the engine snapshots at bind time. Everything
/// else — absolute paths, traversal, unknown prefixes — refuses.
fn resolve_root(paths: &ExecutionPaths, root: &str) -> anyhow::Result<PathBuf> {
    if root == "." {
        return Ok(paths.project_root().to_path_buf());
    }
    if let Some(rel) = mount_path(root, GUEST_WORKSPACES_MOUNT) {
        return Ok(paths.locations().workspaces_root().join(rel));
    }
    if let Some(rel) = mount_path(root, GUEST_STAGING_MOUNT) {
        return Ok(paths.locations().staging_root().join(rel));
    }
    bail!("exec-mode root `{root}` is not a deployment-local tree root")
}

/// The relative path beneath `mount`, when `root` sits strictly under
/// it with plain components only (no `.` / `..` traversal, no
/// re-rooting) — the same discipline as the kernel's workspace-id
/// check, widened to the nested trees ingest stages.
fn mount_path<'a>(root: &'a str, mount: &str) -> Option<&'a str> {
    let rel = root.strip_prefix(mount)?.strip_prefix('/')?;
    let plain = !rel.is_empty()
        && Path::new(rel)
            .components()
            .all(|component| matches!(component, std::path::Component::Normal(_)));
    plain.then_some(rel)
}

/// Refuse anything but a plain `/`-separated relative path.
fn check_rel(path: &str) -> anyhow::Result<()> {
    let plain = !path.is_empty()
        && !path.starts_with('/')
        && path.split('/').all(|segment| !segment.is_empty() && segment != "." && segment != "..");
    if plain { Ok(()) } else { bail!("exec-mode path `{path}` is not a tree-relative path") }
}
