//! Locator materialization (RFC-104 D2): resolve one coverage
//! `location`, snapshot it, and lend a read-only RFC-87 workspace.
//! The observed cid / revision are provenance, never a delivery pin.

use std::path::Path;

use error::Error;
use project::seam::{TreeCredentials, TreeError, TreeLimits, Trees, Workspace, Workspaces};
use project::snapshot::SnapshotId;

/// One materialized coverage location.
#[derive(Debug)]
pub struct Observed {
    /// The lent read-only workspace over the observed tree. The
    /// caller discards it after extract.
    pub workspace: Workspace,
    /// RFC-87 tree identity of the observed snapshot.
    pub cid: SnapshotId,
    /// The commit the fetch reported, when the origin is Git.
    pub revision: Option<String>,
}

/// Materialize one coverage `location`.
///
/// A local tree (relative paths join `home`) snapshots in place; a
/// remote origin fetches through the deployment, snapshots, and
/// discards the fetched tree. Either way the observed snapshot is
/// lent as a read-only workspace.
///
/// # Errors
///
/// `system-source-access` for every failed leg — fetch, snapshot,
/// or preparation. No tree reaches an adapter on failure.
pub async fn materialize(
    workspaces: &impl Workspaces, trees: &impl Trees, home: &Path, location: &str,
) -> Result<Observed, Error> {
    let (cid, revision) = observe(workspaces, trees, home, location).await?;
    let workspace = workspaces
        .prepare(cid.clone(), false)
        .await
        .map_err(|err| access(location, &err.to_string()))?;
    Ok(Observed {
        workspace,
        cid,
        revision,
    })
}

/// Resolve `location` to an observed snapshot plus its revision.
async fn observe(
    workspaces: &impl Workspaces, trees: &impl Trees, home: &Path, location: &str,
) -> Result<(SnapshotId, Option<String>), Error> {
    if project::vcs::is_remote(location) {
        let fetched = trees
            .fetch(location.to_string(), TreeCredentials::Ambient, TreeLimits::unbounded())
            .await
            .map_err(|err: TreeError| access(location, &err.to_string()))?;
        let snapshot = workspaces.snapshot(fetched.root.clone()).await;
        // The fetched tree is transient regardless of snapshot
        // success; discard is best-effort by contract.
        drop(trees.discard_fetched(fetched.root).await);
        Ok((snapshot.map_err(|err| access(location, &err.to_string()))?, fetched.revision))
    } else {
        let path = home.join(location);
        let cid = workspaces
            .snapshot(path.display().to_string())
            .await
            .map_err(|err| access(location, &err.to_string()))?;
        Ok((cid, None))
    }
}

/// The typed access failure a materialization leg maps onto — the
/// survey orchestration records it as `survey-error: access`.
fn access(location: &str, err: &str) -> Error {
    Error::Diag {
        code: "system-source-access",
        detail: format!("materializing `{location}`: {err}"),
    }
}
