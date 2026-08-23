//! Source-adapter Evidence types.

pub mod authority;
pub mod claim;

pub use authority::{AuthorityClass, ClaimKind};
pub use claim::{Backing, Claim, ExampleClaim, validate_claims};
use serde::{Deserialize, Serialize};

/// One kebab-case slug segment (`^[a-z0-9]+(-[a-z0-9]+)*$`).
///
/// This copy remains separate because the leaf adapter SDK cannot
/// depend on `artifacts`.
#[must_use]
pub fn is_kebab(value: &str) -> bool {
    !value.is_empty()
        && value.split('-').all(|segment| {
            !segment.is_empty()
                && segment.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
}

/// A persisted Evidence document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct Document {
    /// Bound lead id.
    pub lead: String,
    /// Document-level authority class for this Evidence.
    pub authority: AuthorityClass,
    /// Extracted claims. An empty set is valid.
    pub claims: Vec<Claim>,
}

impl Document {
    /// Check the lead slug and claims.
    ///
    /// # Errors
    ///
    /// Returns `evidence-schema` validation findings.
    pub fn validate(&self) -> Result<(), emery_error::Error> {
        let mut findings = Vec::new();
        if !is_kebab(&self.lead) {
            findings.push(format!("lead `{}` is not a kebab slug", self.lead));
        }
        findings.extend(validate_claims(&self.claims));
        if findings.is_empty() {
            Ok(())
        } else {
            Err(emery_error::Error::Validation {
                code: "evidence-schema".into(),
                detail: findings.join("; "),
            })
        }
    }
}
