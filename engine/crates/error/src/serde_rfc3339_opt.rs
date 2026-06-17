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

#[cfg(test)]
mod tests {
    use jiff::Timestamp;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
    struct MaybeStamped {
        #[serde(with = "super", default, skip_serializing_if = "Option::is_none")]
        at: Option<Timestamp>,
    }

    #[test]
    fn some_serialises_as_canonical_stamp() {
        let doc = MaybeStamped {
            at: Some("2026-06-02T01:02:03Z".parse().expect("parse")),
        };
        assert_eq!(
            serde_json::to_string(&doc).expect("serialise"),
            r#"{"at":"2026-06-02T01:02:03Z"}"#
        );
    }

    #[test]
    fn none_skipped_with_skip_if() {
        let doc = MaybeStamped { at: None };
        assert_eq!(serde_json::to_string(&doc).expect("serialise"), "{}");
    }

    #[test]
    fn missing_and_null_to_none() {
        let missing: MaybeStamped = serde_json::from_str("{}").expect("missing field");
        assert_eq!(missing, MaybeStamped { at: None });
        let null: MaybeStamped = serde_json::from_str(r#"{"at":null}"#).expect("null field");
        assert_eq!(null, MaybeStamped { at: None });
    }

    #[test]
    fn present_value_deserialises_to_some() {
        let doc: MaybeStamped =
            serde_json::from_str(r#"{"at":"2026-06-02T01:02:03Z"}"#).expect("parse");
        assert_eq!(doc.at, Some("2026-06-02T01:02:03Z".parse().expect("parse")));
    }

    #[test]
    fn present_but_malformed_value_is_rejected() {
        serde_json::from_str::<MaybeStamped>(r#"{"at":"nope"}"#)
            .expect_err("malformed present value is rejected");
    }
}
