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
