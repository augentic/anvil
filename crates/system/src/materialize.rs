//! Locator materialization (RFC-104 D2): resolve one coverage
//! `location`, snapshot it, and lend a read-only RFC-87 workspace.
//! The observed cid / revision are provenance, never a delivery pin.

use std::path::Path;

use error::Error;
use project::seam::{self, Origins, Workspace, Workspaces};
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
    workspaces: &impl Workspaces, origins: &impl Origins, home: &Path, location: &str,
) -> Result<Observed, Error> {
    let (cid, revision) = observe(workspaces, origins, home, location).await?;
    let workspace =
        workspaces.prepare(cid.clone(), false).await.map_err(|err| access(location, &err))?;
    Ok(Observed {
        workspace,
        cid,
        revision,
    })
}

/// Resolve `location` to an observed snapshot plus its revision.
async fn observe(
    workspaces: &impl Workspaces, origins: &impl Origins, home: &Path, location: &str,
) -> Result<(SnapshotId, Option<String>), Error> {
    if project::origins::is_remote(location) {
        let fetched =
            origins.fetch(location.to_string()).await.map_err(|err| access(location, &err))?;
        let snapshot = workspaces.snapshot(fetched.root.clone()).await;
        // The fetched tree is transient regardless of snapshot
        // success; discard is best-effort by contract.
        drop(origins.discard_fetched(fetched.root).await);
        Ok((snapshot.map_err(|err| access(location, &err))?, fetched.revision))
    } else {
        let path = home.join(location);
        let cid = workspaces
            .snapshot(path.display().to_string())
            .await
            .map_err(|err| access(location, &err))?;
        Ok((cid, None))
    }
}

/// The typed access failure a materialization leg maps onto — the
/// survey orchestration records it as `survey-error: access`.
fn access(location: &str, err: &seam::Error) -> Error {
    Error::Diag {
        code: "system-source-access",
        detail: format!("materializing `{location}`: {err}"),
    }
}
