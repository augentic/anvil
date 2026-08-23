//! Evidence answer parsing and validation.

use crate::seam::{ClaimKind, Error, Evidence};

/// Schema for `extract` answers.
pub const EVIDENCE_ANSWER_SCHEMA: &str = include_str!("../schemas/answers/evidence.schema.json");

/// Parses an evidence answer.
///
/// # Errors
///
/// Returns a JSON error if the answer is not [`Evidence`].
pub fn parse_evidence(answer: &str) -> Result<Evidence, serde_json::Error> {
    serde_json::from_str(answer)
}

// The schema accepts strings; enforce grammar here because this leaf
// cannot depend on `emery_artifacts`.
const DOTTED_KEBAB_PATTERN: &str = "^[a-z0-9]+(-[a-z0-9]+)*(\\.[a-z0-9]+(-[a-z0-9]+)*)*$";

fn is_kebab(value: &str) -> bool {
    !value.is_empty()
        && value.split('-').all(|seg| {
            !seg.is_empty() && seg.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
}

fn is_dotted_kebab(value: &str) -> bool {
    !value.is_empty() && value.split('.').all(is_kebab)
}

fn enforce(operation: &str, findings: &[String]) -> Result<(), Error> {
    if findings.is_empty() {
        return Ok(());
    }
    Err(Error::Internal(format!(
        "{operation} answer failed deterministic validation:\n{}",
        findings.join("\n")
    )))
}

/// Enforces required dotted-kebab claim IDs.
///
/// # Errors
///
/// Returns [`Error::Internal`] with one finding per violation.
pub fn validate_evidence(evidence: &Evidence) -> Result<(), Error> {
    let mut findings = Vec::new();
    for (index, claim) in evidence.claims.iter().enumerate() {
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
    enforce("extract", &findings)
}

/// Parses and validates an evidence answer for [`crate::repaired`].
///
/// # Errors
///
/// Returns [`Error::Internal`] on parse or validation failure.
pub fn evidence_tail(answer: &str) -> Result<Evidence, Error> {
    let evidence = parse_evidence(answer)
        .map_err(|err| Error::Internal(format!("evidence answer did not deserialize: {err}")))?;
    validate_evidence(&evidence)?;
    Ok(evidence)
}
