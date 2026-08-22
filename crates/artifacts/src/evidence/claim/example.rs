//! The `kind: example` runtime-capture claim.
//!
//! Bodies larger than 64 `KiB` are stored at `path` with only the digest
//! inline — the cap lives in the adapter brief, not the schema.

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// A `kind: example` claim.
///
/// `input` and `output` remain open for protocol-specific payloads.
/// `replay_digest` is the stable replay anchor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ExampleClaim {
    /// Discriminator, always `example`.
    pub kind: ExampleKind,
    /// Stable claim id.
    pub id: String,
    /// Optional `<path>#L<n>` capture anchor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// `sha256:<hex>` digest of the capture bytes.
    pub replay_digest: String,
    /// Optional open input payload.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<JsonValue>,
    /// Optional open output payload.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<JsonValue>,
    /// Optional single-line behavioural statement for synthesis.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub statement: Option<String>,
}

/// Marker locking `kind` to `example`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, strum::Display)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum ExampleKind {
    /// `kind: example`.
    Example,
}
