//! Patch concerns: the in-memory amend builders.

use super::reconciliation::Divergence;
use super::source::SliceSourceBinding;
use crate::name::SliceName;

/// Three-way patch over a nullable field: `Keep` leaves the field
/// untouched, `Clear` sets it to `None`, `Set(v)` replaces it with
/// `Some(v)`.
///
/// This is the in-memory builder shape consumed by
/// the internal plan amendment kernel; it does **not** appear on the wire.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum Patch<T> {
    /// Leave the field unchanged.
    #[default]
    Keep,
    /// Replace the field with `None`.
    Clear,
    /// Replace the field with `Some(v)`.
    Set(T),
}

impl<T> Patch<T> {
    /// Apply the patch to an `Option<T>` field in place.
    pub(crate) fn apply(self, field: &mut Option<T>) {
        match self {
            Self::Keep => {}
            Self::Clear => *field = None,
            Self::Set(v) => *field = Some(v),
        }
    }
}

impl Patch<String> {
    /// Materialise the wire convention shared by every `Patch<String>`
    /// CLI flag: a missing flag means `Keep`, an empty-string flag
    /// (`--field ""`) means `Clear`, and any non-empty value means
    /// `Set(value)`.
    #[must_use]
    pub fn from_string_option(value: Option<String>) -> Self {
        match value {
            None => Self::Keep,
            Some(s) if s.is_empty() => Self::Clear,
            Some(s) => Self::Set(s),
        }
    }
}

/// Patch applied by the internal plan amendment kernel to an existing entry.
///
/// Wholesale-replacement fields are `Option<Vec<...>>`; nullable fields use
/// the three-way [`Patch`] enum. `status` is deliberately absent —
/// status transitions are made via the internal transition kernel, never
/// through `amend`.
///
/// The absence of a `status` field is a type-system guarantee: `amend`
/// cannot mutate status.
#[derive(Debug, Default, Clone)]
pub struct EntryPatch {
    /// Replace `depends_on` wholesale when `Some`.
    pub depends_on: Option<Vec<SliceName>>,
    /// Replace `sources` wholesale when `Some`.
    pub sources: Option<Vec<SliceSourceBinding>>,
    /// Three-way patch over `project`.
    pub project: Patch<String>,
    /// Three-way patch over `description`.
    pub description: Patch<String>,
    /// Replace `context` wholesale when `Some`.
    pub context: Option<Vec<String>>,
    /// Set `divergence` when `Some`. `None` leaves the field
    /// untouched. The CLI is the only caller that materialises this
    /// patch (`emery plan amend --divergence`) — divergence and writer-ownership contract
    /// widens the accepted operator surface to include `Likely`
    /// alongside `Accepted` / `Rejected`; the implicit `None` value
    /// is still rejected at the flag-parser level (omit
    /// `--divergence` to leave the field alone).
    pub divergence: Option<Divergence>,
}
