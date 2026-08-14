//! Lead-catalog revision retention: copy `leads.md` to `leads/<digest>.md`.

use artifacts::leads::Leads;
use error::Error;

use crate::config::Layout;
use crate::snapshot::SnapshotId;

/// Retain the current `leads.md` at its immutable revision path.
///
/// Computes the canonical digest over parsed content, then copies the
/// exact on-disk document to `leads/<digest>.md` the first time that
/// digest is referenced. A later rewrite of `leads.md` produces a new
/// digest; the retained file is never overwritten.
///
/// # Errors
///
/// Load/parse failures, digest serialization, or filesystem copy
/// failures. `leads-revision-drift` when a retained file exists but
/// no longer matches the current catalog's digest.
pub fn retain(layout: Layout<'_>) -> Result<SnapshotId, Error> {
    let path = layout.leads_path();
    let catalog = Leads::load(&path)?;
    let digest = SnapshotId::from_digest(&catalog.digest_hex()?);
    let dest = layout.leads_revision_path(&digest);
    if dest.exists() {
        let retained = Leads::load(&dest)?;
        if retained.digest_hex()? != digest.digest() {
            return Err(Error::Diag {
                code: "leads-revision-drift",
                detail: format!(
                    "retained catalog at {} no longer matches digest `{digest}`",
                    dest.display()
                ),
            });
        }
        return Ok(digest);
    }
    std::fs::create_dir_all(layout.leads_dir()).map_err(|source| Error::Filesystem {
        op: "mkdir",
        path: layout.leads_dir(),
        source,
    })?;
    std::fs::copy(&path, &dest).map_err(|source| Error::Filesystem {
        op: "copy",
        path: dest,
        source,
    })?;
    Ok(digest)
}
