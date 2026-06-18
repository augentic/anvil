//! RFC 3339 serde helpers: serde_rfc3339 and serde_rfc3339_opt.

mod rfc3339 {
    use jiff::Timestamp;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
    struct Stamped {
        #[serde(with = "specify_error::serde_rfc3339")]
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

mod rfc3339_opt {
    use jiff::Timestamp;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
    struct MaybeStamped {
        #[serde(
            with = "specify_error::serde_rfc3339_opt",
            default,
            skip_serializing_if = "Option::is_none"
        )]
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
