//! runtime capture claim — `kind: example` claim shape (`captures` adapter).
//!
//! Runtime captures join the closed `claimKind` enum as
//! `example`. The body carries `id`, optional `path`, a
//! required `replay-digest: sha256:<hex>`, and the open `input` /
//! `output` JSON-shaped blocks the adapter records from the
//! captured request/response. Bodies larger than 64 `KiB` are stored
//! at `path` with only the digest inline — the cap lives in the
//! adapter brief, not the schema.

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// In-memory shape for a single `kind: example` claim.
///
/// `input` and `output` are deliberately untyped per the schema's
/// open per-kind body posture — the adapter records whatever the
/// captured scenario carried (HTTP method/route/body, message topic /
/// payload shape, scheduled-job arguments). Downstream code consults
/// `replay_digest` as the stable content anchor for replay
/// verification and `path` for the on-disk location of the full body.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ExampleClaim {
    /// Closed claim-kind discriminator. Always serialises as the
    /// literal `example`.
    pub kind: ExampleKind,
    /// Stable claim id (required on `example` per the schema's `allOf`
    /// branch). Resolves the same id space the provenance table joins
    /// against.
    pub id: String,
    /// Optional `<path>#L<n>` anchor (the capture's on-disk location).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// `sha256:<hex>` digest of the capture bytes — the stable content
    /// anchor replay verification joins against.
    pub replay_digest: String,
    /// Optional inline input payload — typically the captured request.
    /// Shape is open per the schema's per-kind body posture.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<JsonValue>,
    /// Optional inline output payload — typically the captured
    /// response plus side-effects. Shape is open.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<JsonValue>,
    /// Optional single-line statement describing the capture's
    /// behavioural meaning (the line synthesis lifts into `spec.md`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub statement: Option<String>,
}

/// Single-variant marker enum locking the `kind:` discriminator to
/// the literal `example` on the wire.
///
/// The variant is the *only* legal value for an [`ExampleClaim`];
/// generic deserialisation through `Claim<ExampleClaim>` would
/// otherwise accept any string. Sibling per-kind claim shapes (when
/// they land) follow the same pattern with their own marker enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, strum::Display)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum ExampleKind {
    /// `kind: example`.
    Example,
}

#[cfg(test)]
mod tests;
