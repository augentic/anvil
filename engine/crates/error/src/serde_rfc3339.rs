//! Serde adapter for `jiff::Timestamp` rendered as RFC 3339.
//!
//! Output is UTC with second precision and a literal `Z` suffix.
//! Use this on `Timestamp` fields; reach for
//! [`crate::serde_rfc3339_opt`] on `Option<Timestamp>` fields. The
//! pairing mirrors `chrono::serde::{ts_seconds, ts_seconds_option}`.

use jiff::Timestamp;
use serde::{Deserialize, Deserializer, Serializer};

/// Serialise a [`Timestamp`] as `%Y-%m-%dT%H:%M:%SZ`. Wired by serde's
/// `with = "specify_error::serde_rfc3339"` attribute.
///
/// # Errors
///
/// Propagates any error produced by the underlying [`Serializer`].
pub fn serialize<S: Serializer>(value: &Timestamp, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.collect_str(&value.strftime("%Y-%m-%dT%H:%M:%SZ"))
}

/// Deserialise a [`Timestamp`] from any rfc3339 string. Accepts
/// `+00:00` and `Z` suffixes alike — the writer canonicalises to `Z`,
/// but pre-canonical fixtures must keep parsing.
///
/// # Errors
///
/// Returns the deserializer's error type when the input is not a
/// well-formed rfc3339 timestamp.
pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Timestamp, D::Error> {
    let raw = String::deserialize(deserializer)?;
    raw.parse().map_err(serde::de::Error::custom)
}
