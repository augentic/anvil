//! Stored lifecycle labels on `metadata.yaml` (non-authority bridge).
//!
//! Progress projects from artifacts and facts (RFC-86 D2).
//! [`LifecycleStatus::transition`] remains for edge validation; phase
//! writers no longer treat the enum as authority.

use error::Error;

/// Lifecycle states a slice passes through.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    strum::Display,
    strum::EnumString,
    strum::VariantNames,
    strum::VariantArray,
)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum LifecycleStatus {
    /// Slice directory created; `/emery:refine` extract + synthesis in flight.
    Refining,
    /// Canonical artifacts validated; ready for `/emery:build`.
    Refined,
    /// Tasks complete; ready for `/emery:merge`.
    Built,
    /// Specs merged into baseline and slice archived.
    Merged,
    /// Slice discarded without merging.
    Dropped,
}

impl LifecycleStatus {
    /// Attempt a transition. Legal edges: `Refining → Refined`,
    /// `Refined → Built`, `Built → Merged`, and
    /// `{Refining, Refined, Built} → Dropped`.
    ///
    /// # Errors
    /// `Error::Diag { code = "slice-lifecycle", .. }` when not
    /// reachable; detail carries the rejected edge verbatim.
    pub fn transition(self, target: Self) -> Result<Self, Error> {
        use LifecycleStatus::{Built, Dropped, Merged, Refined, Refining};
        if matches!(
            (self, target),
            (Refining, Refined)
                | (Refined, Built)
                | (Built, Merged)
                | (Refining | Refined | Built, Dropped)
        ) {
            Ok(target)
        } else {
            Err(Error::Diag {
                code: "slice-lifecycle",
                detail: format!("cannot transition slice from `{self}` to `{target}`"),
            })
        }
    }
}
