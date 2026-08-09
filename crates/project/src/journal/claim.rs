//! Exclusive per-slice claim projection.
//!
//! A slice has at most one live owner; same-slice overlap fails closed
//! as `slice-claim-conflict`. Claims never create build/merge authorization.

use std::collections::BTreeMap;

use error::Error;

use super::{Event, EventKind};
use crate::name::SliceName;

/// Live claim ownership projected from a fact union.
///
/// Keys are slice names; values are the claiming writer ids. Empty when
/// no slice carries a live claim.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Ownership {
    by_slice: BTreeMap<SliceName, String>,
}

impl Ownership {
    /// Writer that currently owns `slice`, when a live claim exists.
    #[must_use]
    pub fn owner(&self, slice: &SliceName) -> Option<&str> {
        self.by_slice.get(slice).map(String::as_str)
    }

    /// Every live `(slice, writer)` pair, ordered by slice name.
    pub fn iter(&self) -> impl Iterator<Item = (&SliceName, &str)> {
        self.by_slice.iter().map(|(slice, writer)| (slice, writer.as_str()))
    }

    /// Number of live claims.
    #[must_use]
    pub fn len(&self) -> usize {
        self.by_slice.len()
    }

    /// Whether no slice is claimed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_slice.is_empty()
    }
}

/// Project live claims from a fact-union already ordered by
/// `(timestamp, writer, sequence)`.
///
/// A `slice.released` clears ownership only when the releasing
/// envelope writer holds the live claim.
#[must_use]
pub fn project(events: &[Event]) -> Ownership {
    let mut ownership = Ownership::default();
    for event in events {
        match &event.kind {
            EventKind::SliceClaimed { slice_name } => {
                ownership.by_slice.insert(slice_name.clone(), event.writer.clone());
            }
            EventKind::SliceReleased { slice_name }
                if ownership
                    .by_slice
                    .get(slice_name)
                    .is_some_and(|owner| owner == &event.writer) =>
            {
                ownership.by_slice.remove(slice_name);
            }
            _ => {}
        }
    }
    ownership
}

/// Ensure `writer` may claim `slice` given current live ownership.
///
/// Same-writer re-claim is idempotent. A live claim by a different
/// writer is `slice-claim-conflict` (exit 2).
///
/// # Errors
///
/// Returns [`Error::Validation`] with code `slice-claim-conflict` when
/// another writer already owns the slice.
pub fn ensure_claimable(
    ownership: &Ownership, slice: &SliceName, writer: &str,
) -> Result<(), Error> {
    match ownership.owner(slice) {
        None => Ok(()),
        Some(owner) if owner == writer => Ok(()),
        Some(owner) => Err(Error::validation_failed(
            "slice-claim-conflict",
            "a slice has at most one owner at a time",
            format!("slice `{slice}` is claimed by `{owner}`; writer `{writer}` cannot claim it"),
        )),
    }
}

/// Return the `slice.claimed` kind after checking exclusivity.
///
/// # Errors
///
/// Same surface as [`ensure_claimable`].
pub fn claim(ownership: &Ownership, slice: SliceName, writer: &str) -> Result<EventKind, Error> {
    ensure_claimable(ownership, &slice, writer)?;
    Ok(EventKind::SliceClaimed { slice_name: slice })
}
