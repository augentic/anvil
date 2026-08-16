//! Protected-input closure algebra (RFC-96 D8): the intersection of
//! descendant covered sets minus patch-touched entries; external
//! protection intersects identical `(id, digest)` oracle entries.

use std::collections::BTreeSet;

use diagnostics::digest::sha256_hex;
use error::Error;
use serde::{Deserialize, Serialize};

use crate::plan::decomposition::{Covered, CoveredKind, Oracle};
use crate::snapshot::SnapshotId;

/// The computed protected-input closure. Empty and absent sets encode
/// as canonical empty sets, so the digest is total.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Closure {
    /// Protected in-tree entries, canonically ordered.
    #[serde(default)]
    pub covered: Vec<Covered>,
    /// Protected external oracles, canonically ordered.
    #[serde(default)]
    pub oracles: Vec<Oracle>,
}

impl Closure {
    /// Content digest of the canonical YAML encoding.
    ///
    /// # Errors
    ///
    /// YAML serialization failures.
    pub fn digest(&self) -> Result<SnapshotId, Error> {
        let yaml = artifacts::atomic::serialise_yaml(self)?;
        Ok(SnapshotId::from_digest(&sha256_hex(yaml.as_bytes())))
    }
}

/// Compute the closure over every descendant's declared sets.
///
/// In-tree: the exact intersection of the per-descendant covered
/// sets, minus any entry a contributing patch touches (a `tree` entry
/// is touched by any path at or under it). External: the intersection
/// of identical `(id, digest)` oracle entries. A descendant with no
/// declaration contributes the empty set, so the intersection is
/// empty.
#[must_use]
pub fn protected_closure(
    descendants: &[(&[Covered], &[Oracle])], touched: &BTreeSet<String>,
) -> Closure {
    let mut iter = descendants.iter();
    let Some((first_covered, first_oracles)) = iter.next() else {
        return Closure::default();
    };
    let mut covered: BTreeSet<Covered> = first_covered.iter().cloned().collect();
    let mut oracles: BTreeSet<Oracle> = first_oracles.iter().cloned().collect();
    for (next_covered, next_oracles) in iter {
        let next: BTreeSet<Covered> = next_covered.iter().cloned().collect();
        covered.retain(|entry| next.contains(entry));
        let next: BTreeSet<Oracle> = next_oracles.iter().cloned().collect();
        oracles.retain(|entry| next.contains(entry));
    }
    covered.retain(|entry| !is_touched(entry, touched));
    Closure {
        covered: covered.into_iter().collect(),
        oracles: oracles.into_iter().collect(),
    }
}

/// Whether any touched path invalidates `entry`.
fn is_touched(entry: &Covered, touched: &BTreeSet<String>) -> bool {
    match entry.kind {
        CoveredKind::File => touched.contains(&entry.path),
        CoveredKind::Tree => touched
            .iter()
            .any(|path| path == &entry.path || path.starts_with(&format!("{}/", entry.path))),
    }
}
