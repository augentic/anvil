//! Serde adapter for `Option<jiff::Timestamp>` rendered as RFC 3339.
//!
//! Companion to [`crate::serde_rfc3339`]; `None` round-trips as `null`
//! (or as a missing field when the call site pairs this with
//! `#[serde(default, skip_serializing_if = "Option::is_none")]`), and
//! `Some` shares the writer/parser of the non-optional variant
//! (`%Y-%m-%dT%H:%M:%SZ` on the wire, any rfc3339 on the way in).

use jiff::Timestamp;
use serde::{Deserialize, Deserializer, Serializer};

/// Serialise `Option<Timestamp>`. `None` → `serialize_none`,
/// `Some(t)` → the canonical second-precision UTC stamp.
///
/// # Errors
///
/// Propagates any error produced by the underlying [`Serializer`].
pub fn serialize<S: Serializer>(
    value: &Option<Timestamp>, serializer: S,
) -> Result<S::Ok, S::Error> {
    match value {
        Some(ts) => serializer.collect_str(&ts.strftime("%Y-%m-%dT%H:%M:%SZ")),
        None => serializer.serialize_none(),
    }
}

/// Deserialise `Option<Timestamp>`; missing or null → `None`,
/// otherwise parsed as rfc3339.
///
/// # Errors
///
/// Returns the deserializer's error type when the input is present
/// but not a well-formed rfc3339 timestamp.
pub fn deserialize<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Timestamp>, D::Error> {
    let opt: Option<String> = Option::deserialize(deserializer)?;
    opt.map(|s| s.parse().map_err(serde::de::Error::custom)).transpose()
}
