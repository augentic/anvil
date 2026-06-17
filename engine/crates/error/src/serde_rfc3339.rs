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

#[cfg(test)]
mod tests {
    use jiff::Timestamp;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
    struct Stamped {
        #[serde(with = "super")]
        at: Timestamp,
    }

    #[test]
    fn serialises_canonical_z_second() {
        let doc = Stamped {
            at: "2026-06-02T01:02:03Z".parse().expect("parse"),
        };
        let json = serde_json::to_string(&doc).expect("serialise");
        assert_eq!(json, r#"{"at":"2026-06-02T01:02:03Z"}"#);
    }

    #[test]
    fn truncates_sub_second() {
        let doc = Stamped {
            at: "2026-06-02T01:02:03.987654Z".parse().expect("parse"),
        };
        let json = serde_json::to_string(&doc).expect("serialise");
        assert_eq!(json, r#"{"at":"2026-06-02T01:02:03Z"}"#, "writer drops to second precision");
    }

    #[test]
    fn z_and_offset_same_instant() {
        let z: Stamped =
            serde_json::from_str(r#"{"at":"2026-06-02T01:02:03Z"}"#).expect("parse Z form");
        let offset: Stamped = serde_json::from_str(r#"{"at":"2026-06-02T01:02:03+00:00"}"#)
            .expect("parse offset form");
        assert_eq!(z, offset, "pre-canonical +00:00 fixtures parse to the same instant as Z");
    }

    #[test]
    fn rejects_non_rfc3339_input() {
        let err = serde_json::from_str::<Stamped>(r#"{"at":"not-a-timestamp"}"#)
            .expect_err("garbage timestamp is rejected");
        assert!(err.to_string().contains("at") || !err.to_string().is_empty());
    }
}
