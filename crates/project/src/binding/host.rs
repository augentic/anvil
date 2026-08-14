//! Native-only Git clone and HTTPS fetch, plus CID snapshot.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use error::Error;

use super::git::checkout;
use super::https_fetch::fetch as fetch_https;
use super::ingest::{Resolved, Session, Staged};
use super::locator::{Location, Locator};
use crate::snapshot::SnapshotId;
use crate::workspace::Objects;

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Host ingest of one locator: Git/HTTPS I/O then the wasm-clean CID kernel.
///
/// Returns the resolved pin and the staged tree path for fingerprinting.
///
/// # Errors
///
/// Locator, Git, HTTPS, budget, and snapshot failures.
pub async fn resolve<O: Objects>(
    session: &mut Session<'_, O>, location: &Location, recorded: Option<&SnapshotId>,
    prior: Option<&str>, scratch: &Path,
) -> Result<(Resolved, PathBuf), Error> {
    match &location.locator {
        Locator::Path(_) => {
            let resolved = session.ingest(location, Staged::Disk, recorded, None).await?;
            let root = path_root(session.change_root, location)?;
            Ok((resolved, root))
        }
        Locator::Git { url, revision } => {
            if let Some(cid) = recorded
                && session.store.contains(cid).await
            {
                let resolved = session.ingest(location, Staged::Disk, recorded, None).await?;
                return Ok((
                    resolved,
                    path_root(session.change_root, location)
                        .unwrap_or_else(|_| scratch.to_path_buf()),
                ));
            }
            if let Some(cid) = session.cache.get(location)
                && session.store.contains(&cid).await
            {
                let resolved = session.ingest(location, Staged::Disk, Some(&cid), None).await?;
                return Ok((resolved, scratch.to_path_buf()));
            }
            let dest = scratch.join(format!("git-{}", NEXT.fetch_add(1, Ordering::Relaxed)));
            let exact = checkout(url, revision, prior, &dest, session.policy, session.meter)?;
            let mut pinned = location.clone();
            pinned.locator = Locator::Git {
                url: url.clone(),
                revision: exact.revision,
            };
            let resolved =
                session.ingest(&pinned, Staged::Tree(&dest), recorded, exact.warning).await?;
            session.cache.insert(location, resolved.cid.clone());
            Ok((resolved, dest))
        }
        Locator::Https(url) => {
            if let Some(cid) = recorded
                && session.store.contains(cid).await
            {
                let resolved = session.ingest(location, Staged::Disk, recorded, None).await?;
                return Ok((resolved, scratch.to_path_buf()));
            }
            if let Some(cid) = session.cache.get(location)
                && session.store.contains(&cid).await
            {
                let resolved = session.ingest(location, Staged::Disk, Some(&cid), None).await?;
                return Ok((resolved, scratch.to_path_buf()));
            }
            let bytes = fetch_https(url, session.policy, session.meter)?;
            let resolved = session.ingest(location, Staged::Bytes(&bytes), recorded, None).await?;
            Ok((resolved, scratch.to_path_buf()))
        }
    }
}

fn path_root(change_root: &Path, location: &Location) -> Result<PathBuf, Error> {
    let Locator::Path(path) = &location.locator else {
        return Err(Error::Diag {
            code: "locator-malformed",
            detail: "disk ingest requires a path locator".into(),
        });
    };
    let base = if path.is_absolute() { path.clone() } else { change_root.join(path) };
    if location.path == "." {
        return Ok(base);
    }
    Ok(base.join(&location.path))
}

/// Fetch a locator string into a seam [`crate::seam::Fetched`].
///
/// # Errors
///
/// Locator parse, Git, HTTPS, budget, and snapshot failures.
pub async fn fetch<O: Objects>(
    session: &mut Session<'_, O>, locator: &str, recorded: Option<&SnapshotId>, prior: Option<&str>,
) -> Result<crate::seam::Fetched, Error> {
    let location = Location::parse(locator, None)?;
    let scratch = session.scratch;
    let (resolved, root) = resolve(session, &location, recorded, prior, scratch).await?;
    Ok(crate::seam::Fetched {
        locator: resolved.location.locator.key(),
        cid: resolved.cid,
        root: root.display().to_string(),
        warning: resolved.warning,
    })
}
