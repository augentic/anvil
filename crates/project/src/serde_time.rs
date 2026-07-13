//! RFC 3339 serde adapters for workflow timestamps.

/// `#[serde(with = "…")]` adapter for a required [`jiff::Timestamp`].
pub mod rfc3339 {
    use jiff::Timestamp;
    use serde::{Deserialize, Deserializer, Serializer};

    /// Serialize as `YYYY-MM-DDTHH:MM:SSZ`.
    ///
    /// # Errors
    ///
    /// Propagates the serializer's failure.
    pub fn serialize<S: Serializer>(value: &Timestamp, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(&value.strftime("%Y-%m-%dT%H:%M:%SZ"))
    }

    /// Parse an RFC 3339 string.
    ///
    /// # Errors
    ///
    /// A custom deserialization error when the string does not parse.
    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Timestamp, D::Error> {
        let raw = String::deserialize(deserializer)?;
        raw.parse().map_err(serde::de::Error::custom)
    }
}

/// `#[serde(with = "…")]` adapter for an optional [`jiff::Timestamp`].
pub mod rfc3339_opt {
    use jiff::Timestamp;
    use serde::{Deserialize, Deserializer, Serializer};

    /// Serialize as `YYYY-MM-DDTHH:MM:SSZ`, or `null` when absent.
    ///
    /// # Errors
    ///
    /// Propagates the serializer's failure.
    pub fn serialize<S: Serializer>(
        value: &Option<Timestamp>, serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match value {
            Some(timestamp) => serializer.collect_str(&timestamp.strftime("%Y-%m-%dT%H:%M:%SZ")),
            None => serializer.serialize_none(),
        }
    }

    /// Parse an optional RFC 3339 string.
    ///
    /// # Errors
    ///
    /// A custom deserialization error when a present string does not parse.
    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<Timestamp>, D::Error> {
        let value: Option<String> = Option::deserialize(deserializer)?;
        value.map(|raw| raw.parse().map_err(serde::de::Error::custom)).transpose()
    }
}
