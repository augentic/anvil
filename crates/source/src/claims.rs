//! Deterministic claim validation shared by the engine's extract gate
//! and the SDK's answer tail.

use crate::types::{Claim, ClaimKind, Error, Evidence};

/// Claim-id grammar; the answer schema accepts strings, so it is
/// enforced in code.
pub const DOTTED_KEBAB_PATTERN: &str = "^[a-z0-9]+(-[a-z0-9]+)*(\\.[a-z0-9]+(-[a-z0-9]+)*)*$";

fn is_kebab(value: &str) -> bool {
    !value.is_empty()
        && value.split('-').all(|seg| {
            !seg.is_empty() && seg.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
}

fn is_dotted_kebab(value: &str) -> bool {
    !value.is_empty() && value.split('.').all(is_kebab)
}

/// Findings for dotted-kebab claim ids and required-id kinds.
#[must_use]
pub fn claim_id_findings(claims: &[Claim]) -> Vec<String> {
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
                findings.push(format!("- claim {index}: `{:?}` claims require an id", claim.kind));
            }
            _ => {}
        }
    }
    findings
}

/// Enforces required dotted-kebab claim IDs.
///
/// # Errors
///
/// Returns [`Error::Internal`] with one finding per violation.
pub fn validate_evidence(evidence: &Evidence) -> Result<(), Error> {
    let findings = claim_id_findings(&evidence.claims);
    if findings.is_empty() {
        return Ok(());
    }
    Err(Error::Internal(format!(
        "extract answer failed deterministic validation:\n{}",
        findings.join("\n")
    )))
}
