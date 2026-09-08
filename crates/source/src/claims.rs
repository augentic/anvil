//! Claim rules
//!
//! What makes a claim well formed: the grammar its id must follow, which
//! claim kinds must carry an id at all, and the extra fields each kind is
//! required to include. [`Evidence::validate`] applies all of them to a whole
//! document.
//!
//! The rules live in the contract crate because two parties enforce them.
//! An adapter checks its own answer so a bad claim can be repaired before it
//! leaves the guest; the engine checks again on receipt, because it cannot
//! assume every adapter did.

use crate::types::{Claim, ClaimKind, Error, Evidence};

/// Claim-id grammar; the answer schema accepts strings, so it is
/// enforced in code.
pub const DOTTED_KEBAB_PATTERN: &str = "^[a-z0-9]+(-[a-z0-9]+)*(\\.[a-z0-9]+(-[a-z0-9]+)*)*$";

/// Every id and extras finding over `claims`.
#[must_use]
pub fn findings(claims: &[Claim]) -> Vec<String> {
    let mut findings = id_findings(claims);
    findings.extend(extras_findings(claims));
    findings
}

/// Findings for dotted-kebab claim ids and required-id kinds.
#[must_use]
pub fn id_findings(claims: &[Claim]) -> Vec<String> {
    let mut findings = Vec::new();
    for (index, claim) in claims.iter().enumerate() {
        match &claim.id {
            Some(id) if !is_dotted_kebab(id) => {
                findings.push(format!(
                    "- claim {index}: id `{id}` does not match `{DOTTED_KEBAB_PATTERN}`"
                ));
            }
            None if matches!(
                claim.kind,
                ClaimKind::Requirement | ClaimKind::Criterion | ClaimKind::Example
            ) =>
            {
                let kind = claim.kind;
                findings.push(format!("- claim {index}: `{kind}` claims require an id"));
            }
            _ => {}
        }
    }
    findings
}

/// Findings for absent required per-kind extras.
#[must_use]
pub fn extras_findings(claims: &[Claim]) -> Vec<String> {
    let mut findings = Vec::new();
    for (index, claim) in claims.iter().enumerate() {
        for key in claim.kind.required_extras() {
            if !claim.extras.contains_key(*key) {
                let label = claim.id.as_deref().unwrap_or("<unnamed>");
                let kind = claim.kind;
                findings
                    .push(format!("- claim {index}: `{kind}` `{label}` is missing extra `{key}`"));
            }
        }
    }
    findings
}

impl Evidence {
    /// Enforces claim-id grammar and required extras fail-closed.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Internal`] with one finding per violation.
    pub fn validate(&self) -> Result<(), Error> {
        let findings = findings(&self.claims);
        if findings.is_empty() {
            return Ok(());
        }
        let findings = findings.join("\n");
        Err(Error::Internal(format!("invalid claims:\n{findings}")))
    }
}

impl ClaimKind {
    /// Returns the extras this kind must carry.
    ///
    /// Widening this closed table is a contract change.
    #[must_use]
    pub const fn required_extras(self) -> &'static [&'static str] {
        match self {
            Self::Requirement => &["statement"],
            Self::Criterion => &["criterion"],
            Self::Example => &["replay-digest"],
            _ => &[],
        }
    }
}

impl Claim {
    /// The `statement` extra; empty when absent.
    ///
    /// The extract gate guarantees a requirement carries this extra.
    #[must_use]
    pub fn statement(&self) -> String {
        match self.extras.get("statement") {
            Some(serde_json::Value::String(text)) => text.clone(),
            Some(other) => other.to_string(),
            None => String::new(),
        }
    }

    /// The claim's id, or its path when it has no id.
    #[must_use]
    pub fn type_key(&self) -> Option<&str> {
        self.id.as_deref().or(self.path.as_deref())
    }

    /// The `signature` extra, when it is a string.
    #[must_use]
    pub fn signature(&self) -> Option<&str> {
        match self.extras.get("signature") {
            Some(serde_json::Value::String(signature)) => Some(signature),
            _ => None,
        }
    }
}

fn is_dotted_kebab(value: &str) -> bool {
    !value.is_empty() && value.split('.').all(is_kebab)
}

fn is_kebab(value: &str) -> bool {
    !value.is_empty()
        && value.split('-').all(|seg| {
            !seg.is_empty() && seg.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
}
