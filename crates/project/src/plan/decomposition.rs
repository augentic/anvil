//! Conflict-domain hierarchy: closed `decomposition.yaml`, validators,
//! domain-dependency compiler, leaf projector, and revision retention.

mod compile;
mod contraction;
mod judgment;
mod project;
mod tree;
mod validate;

pub use compile::edges as compile;
pub use contraction::cycles as contraction;
use error::Error;
pub use judgment::{
    BoundaryReview, Child, FocusParent, PARTITION_VERSION, PartitionKind, PartitionResponse,
    ReviewVerdict,
};
pub use project::{matches_plan, slices};
pub use tree::{
    BoundProfile, Decomposition, Kind, MAX_DEPTH, MAX_JUDGMENTS, MAX_NODES, Node, Scope, VERSION,
};
pub use validate::findings;

use crate::config::Layout;
use crate::snapshot::SnapshotId;

/// Retain the current `decomposition.yaml` at its immutable revision path.
///
/// Digests cover canonical YAML. The first reference copies the exact
/// on-disk document; later rewrites produce a new digest and the
/// retained file is never overwritten.
///
/// # Errors
///
/// Load/parse failures, digest serialization, or filesystem copy
/// failures. `decomposition-revision-drift` when a retained file
/// exists but no longer matches the current document's digest.
pub fn retain(layout: Layout<'_>) -> Result<SnapshotId, Error> {
    let path = layout.decomposition_path();
    let tree = Decomposition::load(&path)?;
    let digest = tree.digest()?;
    let dest = layout.decomp_revision_path(&digest);
    if dest.exists() {
        let retained = Decomposition::load(&dest)?;
        if retained.digest()? != digest {
            return Err(Error::Diag {
                code: "decomposition-revision-drift",
                detail: format!(
                    "retained decomposition at {} no longer matches digest `{digest}`",
                    dest.display()
                ),
            });
        }
        return Ok(digest);
    }
    std::fs::create_dir_all(layout.decompositions_dir()).map_err(|source| Error::Filesystem {
        op: "mkdir",
        path: layout.decompositions_dir(),
        source,
    })?;
    std::fs::copy(&path, &dest).map_err(|source| Error::Filesystem {
        op: "copy",
        path: dest,
        source,
    })?;
    Ok(digest)
}
