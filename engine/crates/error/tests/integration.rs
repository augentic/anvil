#![allow(clippy::too_many_lines)]

mod lib {
    use specify_error::{is_kebab, is_kebab_leading_alpha};

    #[test]
    fn is_kebab_accepts_and_rejects() {
        for ok in ["a", "abc", "alpha-gateway", "x-1", "a1-b2"] {
            assert!(is_kebab(ok), "expected `{ok}` to pass");
        }
        for bad in ["", "-a", "a-", "a--b", "A", "alpha_gateway", "alpha gateway"] {
            assert!(!is_kebab(bad), "expected `{bad}` to fail");
        }
    }

    #[test]
    fn is_kebab_leading_alpha_rejects() {
        for ok in ["a", "tab-bar", "x-1"] {
            assert!(is_kebab_leading_alpha(ok), "expected `{ok}` to pass");
        }
        for bad in ["", "1a", "9-lives", "-a", "a--b", "A"] {
            assert!(!is_kebab_leading_alpha(bad), "expected `{bad}` to fail");
        }
    }
}

mod error {
    use specify_error::Error;

    #[test]
    fn diag_round_trip() {
        let err = Error::Diag {
            code: "kebab-prefix",
            detail: "specific detail".to_string(),
        };
        assert_eq!(err.variant_str(), "kebab-prefix");
        assert_eq!(err.to_string(), "kebab-prefix: specific detail");
    }

    #[test]
    fn cli_too_old_discriminant_display() {
        let err = Error::CliTooOld {
            required: "1.0.0".to_string(),
            found: "0.9.0".to_string(),
        };
        assert_eq!(err.variant_str(), "specify-version-too-old");
        let msg = err.to_string();
        assert!(msg.contains("0.9.0") && msg.contains("1.0.0"), "both versions in display: {msg}");
        assert!(err.hint().is_none(), "CliTooOld has no recovery hint");
    }

    #[test]
    fn adapter_cli_too_old_discriminant_display() {
        // RFC-47 D3: the adapter floor reuses the exit-3 family but carries
        // a distinct discriminant naming the adapter that outran the binary.
        let err = Error::AdapterCliTooOld {
            adapter: "omnia (adapter.yaml)".to_string(),
            required: "2.0.0".to_string(),
            found: "1.0.0".to_string(),
        };
        assert_eq!(err.variant_str(), "adapter-cli-too-old");
        let msg = err.to_string();
        assert!(
            msg.contains("1.0.0") && msg.contains("2.0.0") && msg.contains("omnia"),
            "versions and adapter in display: {msg}"
        );
        assert!(err.hint().is_none(), "AdapterCliTooOld has no recovery hint");
    }

    #[test]
    fn validation_static_code_and_display() {
        // The common path borrows a `&'static str` code, and
        // `validation_failed` folds `rule` + `detail` into one message.
        let err = Error::validation_failed("bad-thing", "rule", "detail");
        assert_eq!(err.variant_str(), "bad-thing");
        assert_eq!(err.to_string(), "bad-thing: rule: detail");
    }

    #[test]
    fn validation_empty_rule_omits_prefix() {
        // Edge: an empty `rule` must not leave a dangling `": "` prefix.
        let err = Error::validation_failed("code", "", "just detail");
        assert_eq!(err.to_string(), "code: just detail");
        assert_eq!(err.variant_str(), "code");
    }
}

mod codes {
    use specify_error::codes::WIRE_CODES;

    #[test]
    fn sorted_unique_kebab() {
        for window in WIRE_CODES.windows(2) {
            assert!(
                window[0] < window[1],
                "WIRE_CODES must stay sorted and deduplicated: `{}` >= `{}`",
                window[0],
                window[1]
            );
        }
        for code in WIRE_CODES {
            assert!(specify_error::is_kebab(code), "wire code `{code}` must be kebab-case");
        }
    }
}

mod serde_rfc3339 {
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

mod serde_rfc3339_opt {
    use jiff::Timestamp;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
    struct MaybeStamped {
        #[serde(with = "specify_error::serde_rfc3339_opt", default, skip_serializing_if = "Option::is_none")]
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
