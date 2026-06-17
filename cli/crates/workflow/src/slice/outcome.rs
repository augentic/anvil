//! Phase-outcome discriminant. The wire-format wrapper that pairs an
//! [`Kind`] with phase + timestamp metadata lives in
//! [`crate::slice::metadata`] as `Outcome`.

use serde::{Deserialize, Serialize};

/// Phase outcome reported to `/spec:execute`. Unit variants serialise
/// as `outcome: success` etc.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, strum::Display)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum Kind {
    /// Phase completed successfully.
    Success,
    /// Phase failed.
    Failure,
    /// Phase deferred (needs human input).
    Deferred,
}

#[cfg(test)]
mod tests;
