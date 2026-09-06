//! Deterministic claim validation shared by the engine's extract gate
//! and the SDK's answer tail: claim-id grammar and the required
//! per-kind extras (A8).

use crate::types::{Claim, ClaimKind, Error, Evidence};

/// Claim-id grammar; the answer schema accepts strings, so it is
/// enforced in code.
pub const DOTTED_KEBAB_PATTERN: &str = "^[a-z0-9]+(-[a-z0-9]+)*(\\.[a-z0-9]+(-[a-z0-9]+)*)*$";

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
        Err(Error::Internal(format!(
            "extract answer failed deterministic validation:\n{}",
            findings.join("\n")
        )))
    }
}

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
                findings.push(format!("- claim {index}: `{}` claims require an id", claim.kind));
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
                findings.push(format!(
                    "- claim {index}: `{}` claim `{label}` is missing its required `{key}` extra",
                    claim.kind
                ));
            }
        }
    }
    findings
}

fn is_kebab(value: &str) -> bool {
    !value.is_empty()
        && value.split('-').all(|seg| {
            !seg.is_empty() && seg.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
}

fn is_dotted_kebab(value: &str) -> bool {
    !value.is_empty() && value.split('.').all(is_kebab)
}
