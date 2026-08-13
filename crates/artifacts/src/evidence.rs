//! Source-adapter Evidence shapes.
//!
//! Per-source `extract` output at `.emery/change/slices/<slice>/evidence/<source>.yaml`:
//! typed [`Document`] / [`Claim`] plus the closed [`AuthorityClass`] / [`ClaimKind`] enums.

pub mod authority;
pub mod claim;

pub use authority::{AuthorityClass, ClaimKind};
pub use claim::{Backing, Claim, ExampleClaim, validate_claims};
use serde::{Deserialize, Serialize};

/// One kebab-case slug segment (`^[a-z0-9]+(-[a-z0-9]+)*$`).
#[must_use]
pub fn is_kebab(value: &str) -> bool {
    !value.is_empty()
        && value.split('-').all(|segment| {
            !segment.is_empty()
                && segment.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
}

/// The full persisted Evidence document: the envelope `lead` key plus
/// the extract answer's `authority` and `claims`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct Document {
    /// Lead id (matches a `## Lead inventory` block in `discovery.md`)
    /// the Evidence is bound to.
    pub lead: String,
    /// Document-level authority class for this Evidence.
    pub authority: AuthorityClass,
    /// Per-claim extraction records. Empty `claims: []` is valid;
    /// failure is signalled by not writing the Evidence file at all.
    pub claims: Vec<Claim>,
}

impl Document {
    /// Deterministically re-check the document: the `lead` must be a
    /// kebab slug and the claim set must pass [`validate_claims`].
    ///
    /// # Errors
    ///
    /// Returns [`error::Error::Validation`] keyed on `evidence-schema`
    /// (exit code 2) carrying one line per violation.
    pub fn validate(&self) -> Result<(), error::Error> {
        let mut findings = Vec::new();
        if !is_kebab(&self.lead) {
            findings.push(format!("lead `{}` is not a kebab slug", self.lead));
        }
        findings.extend(validate_claims(&self.claims));
        if findings.is_empty() {
            Ok(())
        } else {
            Err(error::Error::Validation {
                code: "evidence-schema".into(),
                detail: findings.join("; "),
            })
        }
    }
}
