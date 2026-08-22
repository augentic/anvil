//! Typed Evidence claims.
//!
//! [`Claim`] mirrors the WIT `claim` record; open per-kind body fields
//! survive round-trips through the flattened [`Claim::extras`] map.

pub mod example;

pub use example::ExampleClaim;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use super::authority::ClaimKind;

/// Claim backing matching the WIT `backing` variant.
///
/// On the wire the variant flattens onto the claim object: an inline
/// payload serialises as `payload`, a filesystem pointer as
/// `backing-path` (distinct from the claim anchor's `path`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Backing {
    /// A small, verbatim piece of data passed directly.
    Payload(String),
    /// A pointer to a block of data in the filesystem.
    Path(String),
}

/// One extracted Evidence claim.
///
/// The shape stays open (`extras` flattens unmodeled keys) because
/// per-kind body fields are deliberately unconstrained — a closed
/// mirror could drop data that synthesis reads verbatim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct Claim {
    /// The claim's kind from the closed taxonomy.
    pub kind: ClaimKind,
    /// Dotted-kebab id, required for requirement, criterion, and example claims.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Per-claim source anchor: `<path>`, `<path>#L<n>`, or
    /// `<path>#L<start>-L<end>`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Semantic synopsis.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub synopsis: Option<String>,
    /// Inline backing payload ([`Backing::Payload`] on the wire).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<String>,
    /// Filesystem backing pointer ([`Backing::Path`] on the wire).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backing_path: Option<String>,
    /// Open per-kind body fields, carried verbatim.
    #[serde(flatten)]
    pub extras: serde_json::Map<String, JsonValue>,
}

impl Claim {
    /// A claim with only its `kind` set.
    #[must_use]
    pub fn new(kind: ClaimKind) -> Self {
        Self {
            kind,
            id: None,
            path: None,
            synopsis: None,
            payload: None,
            backing_path: None,
            extras: serde_json::Map::new(),
        }
    }

    /// Return WIT-shaped backing; inline payload wins if both forms exist.
    #[must_use]
    pub fn backing(&self) -> Option<Backing> {
        if let Some(payload) = &self.payload {
            return Some(Backing::Payload(payload.clone()));
        }
        self.backing_path.clone().map(Backing::Path)
    }

    /// Set the backing from the WIT-shaped variant.
    pub fn set_backing(&mut self, backing: Option<Backing>) {
        self.payload = None;
        self.backing_path = None;
        match backing {
            Some(Backing::Payload(payload)) => self.payload = Some(payload),
            Some(Backing::Path(path)) => self.backing_path = Some(path),
            None => {}
        }
    }
}

/// Claim-id grammar: a dotted kebab slug
/// (`^[a-z0-9]+(-[a-z0-9]+)*(\.[a-z0-9]+(-[a-z0-9]+)*)*$`).
#[must_use]
pub fn is_dotted_kebab(value: &str) -> bool {
    !value.is_empty() && value.split('.').all(super::is_kebab)
}

/// Return id-grammar and required-id findings for a claim set.
#[must_use]
pub fn validate_claims(claims: &[Claim]) -> Vec<String> {
    let mut findings = Vec::new();
    for (index, claim) in claims.iter().enumerate() {
        match &claim.id {
            Some(id) if !is_dotted_kebab(id) => {
                findings.push(format!("claim {index}: id `{id}` is not a dotted kebab slug"));
            }
            None if matches!(
                claim.kind,
                ClaimKind::Requirement | ClaimKind::Criterion | ClaimKind::Example
            ) =>
            {
                findings.push(format!("claim {index}: `{}` claims require an id", claim.kind));
            }
            _ => {}
        }
    }
    findings
}
