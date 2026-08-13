//! Host-side Git clone and HTTPS fetch for locator ingest.

mod git;
mod https;

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use error::Error;
pub use git::checkout;
pub use https::fetch;
use project::binding::{Location, Locator, Resolved, Session, Staged};
use project::snapshot::SnapshotId;
use project::workspace::Objects;

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Host ingest of one locator: Git/HTTPS I/O then the wasm-clean CID kernel.
///
/// # Errors
///
/// Locator, Git, HTTPS, budget, and snapshot failures.
pub async fn resolve<O: Objects>(
    session: &mut Session<'_, O>, location: &Location, recorded: Option<&SnapshotId>,
    prior: Option<&str>, scratch: &Path,
) -> Result<Resolved, Error> {
    match &location.locator {
        Locator::Path(_) => session.ingest(location, Staged::Disk, recorded, None).await,
        Locator::Git { url, revision } => {
            if let Some(cid) = recorded
                && session.store.contains(cid).await
            {
                return session.ingest(location, Staged::Disk, recorded, None).await;
            }
            if let Some(cid) = session.cache.get(location)
                && session.store.contains(&cid).await
            {
                return session.ingest(location, Staged::Disk, Some(&cid), None).await;
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
            Ok(resolved)
        }
        Locator::Https(url) => {
            if let Some(cid) = recorded
                && session.store.contains(cid).await
            {
                return session.ingest(location, Staged::Disk, recorded, None).await;
            }
            if let Some(cid) = session.cache.get(location)
                && session.store.contains(&cid).await
            {
                return session.ingest(location, Staged::Disk, Some(&cid), None).await;
            }
            let bytes = fetch(url, session.policy, session.meter)?;
            session.ingest(location, Staged::Bytes(&bytes), recorded, None).await
        }
    }
}
