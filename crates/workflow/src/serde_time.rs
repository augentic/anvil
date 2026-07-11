//! RFC 3339 serde adapters for workflow timestamps.

pub mod rfc3339 {
    use jiff::Timestamp;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(value: &Timestamp, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(&value.strftime("%Y-%m-%dT%H:%M:%SZ"))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Timestamp, D::Error> {
        let raw = String::deserialize(deserializer)?;
        raw.parse().map_err(serde::de::Error::custom)
    }
}

pub mod rfc3339_opt {
    use jiff::Timestamp;
    use serde::{Deserialize, Deserializer, Serializer};

    #[expect(clippy::ref_option, reason = "serde `with` adapters receive a reference to the field")]
    pub fn serialize<S: Serializer>(
        value: &Option<Timestamp>, serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match value {
            Some(timestamp) => serializer.collect_str(&timestamp.strftime("%Y-%m-%dT%H:%M:%SZ")),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<Timestamp>, D::Error> {
        let value: Option<String> = Option::deserialize(deserializer)?;
        value.map(|raw| raw.parse().map_err(serde::de::Error::custom)).transpose()
    }
}

#[cfg(test)]
mod tests {
    use jiff::Timestamp;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
    struct Stamped {
        #[serde(with = "super::rfc3339")]
        at: Timestamp,
        #[serde(with = "super::rfc3339_opt", default, skip_serializing_if = "Option::is_none")]
        optional: Option<Timestamp>,
    }

    #[test]
    fn round_trip() {
        let stamp = Stamped {
            at: "2026-06-02T01:02:03.987654Z".parse().expect("timestamp parses"),
            optional: None,
        };
        let json = serde_json::to_string(&stamp).expect("stamp serialises");
        assert_eq!(json, r#"{"at":"2026-06-02T01:02:03Z"}"#);

        let offset: Stamped =
            serde_json::from_str(r#"{"at":"2026-06-02T01:02:03+00:00","optional":null}"#)
                .expect("offset timestamp parses");
        assert_eq!(offset.at, "2026-06-02T01:02:03Z".parse().expect("timestamp parses"));
        assert_eq!(offset.optional, None);
    }

    #[test]
    fn rejects_malformed() {
        serde_json::from_str::<Stamped>(r#"{"at":"not-a-timestamp"}"#)
            .expect_err("malformed timestamp is rejected");
    }
}
