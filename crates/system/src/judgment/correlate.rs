//! The correlation judgment leg (RFC-104 D4): the complete extracted
//! Evidence set composes into the `as-is` state. Identities, `target`,
//! `transition-*`, and `decisions/` never enter the envelope.

use std::collections::BTreeSet;

use error::Error;
use omnia_guest::Model;
use serde::{Deserialize, Serialize};

use super::{prose, render_json, repaired};
use crate::model::{State, Status};

/// Wire version stamped on both envelopes.
const CORRELATION_VERSION: u32 = 1;

/// Correlation input envelope kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum InputKind {
    /// Agent correlation inputs.
    Inputs,
}

/// The correlation step's input envelope.
///
/// Lists the included `(source, lead)` Evidence paths the agent reads
/// from the lent definition-home tree, plus the operator's declared
/// decision for orientation. Deliberately absent: `identities[]`,
/// `target`, `transition-*`, and `decisions/` (the persist tail
/// reapplies them after the answer).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct CorrelationInputs {
    /// Schema version.
    pub version: u32,
    /// Envelope kind.
    pub kind: InputKind,
    /// The decision the survey supports (`scope.yaml.decision`).
    pub decision: String,
    /// One entry per included `(source, lead)` Evidence document.
    pub evidence: Vec<EvidenceRef>,
}

/// One surface-grain Evidence document reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub struct EvidenceRef {
    /// Coverage-row source key.
    pub source: String,
    /// The surveyed lead id.
    pub lead: String,
    /// Home-relative path to the document
    /// (`evidence/<source>/<lead>.yaml`).
    pub evidence_path: String,
}

/// Correlation response envelope kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ResponseKind {
    /// The agent's correlation result.
    Response,
}

/// `kind: response` envelope — the composed `as-is` state only.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct CorrelationResponse {
    /// Schema version.
    pub version: u32,
    /// Envelope kind.
    pub kind: ResponseKind,
    /// The composed `as-is` element and relationship set.
    pub as_is: State,
}

/// Assemble the correlation input envelope.
#[must_use]
pub fn inputs(decision: &str, evidence: Vec<EvidenceRef>) -> CorrelationInputs {
    CorrelationInputs {
        version: CORRELATION_VERSION,
        kind: InputKind::Inputs,
        decision: decision.to_string(),
        evidence,
    }
}

/// Run the correlation judgment leg over an assembled inputs envelope.
///
/// `claims` is the `(source, id)` anchor index over the persisted
/// Evidence corpus; the deterministic tail closes every cited claim
/// against it inside the repair loop.
///
/// # Errors
///
/// The mapped model failure, or the last schema / parse / validation
/// failure once the repair budget is exhausted.
pub async fn correlate<P: Model>(
    model: &P, inputs: &CorrelationInputs, claims: &BTreeSet<(String, String)>,
) -> Result<State, Error> {
    let schema = project::answers::render(&crate::answers::correlation());
    let user = format!(
        "## Correlation inputs\n\n```json\n{}\n```",
        render_json(inputs, "correlation inputs")?
    );
    repaired(
        model,
        prose::correlate_system(),
        user,
        "correlation",
        &schema,
        project::judgment::Lent::default(),
        |answer| {
            let response: CorrelationResponse = serde_saphyr::from_str(answer).map_err(|err| {
                Error::validation_failed(
                    "system-correlate-response-parse",
                    "the answer deserialises as a correlation response",
                    format!("failed to parse correlation response: {err}"),
                )
            })?;
            tail(response.as_is, claims)
        },
    )
    .await
}

/// The deterministic tail: structural validation plus provenance
/// closure against the persisted Evidence corpus.
fn tail(state: State, claims: &BTreeSet<(String, String)>) -> Result<State, Error> {
    state.validate("as-is")?;
    let cited =
        state.elements.iter().map(|element| (&element.id, element.status, &element.claims)).chain(
            state
                .relationships
                .iter()
                .map(|relationship| (&relationship.id, relationship.status, &relationship.claims)),
        );
    for (id, status, refs) in cited {
        if status == Status::Decided {
            return Err(Error::validation_failed(
                "system-correlate-decided",
                "correlation cannot decide",
                format!("`{id}` answered with `status: decided`; only `decisions/` records decide"),
            ));
        }
        for claim in refs {
            if !claims.contains(&(claim.source.clone(), claim.id.clone())) {
                return Err(Error::validation_failed(
                    "system-correlate-claims",
                    "every cited claim exists in the Evidence corpus",
                    format!(
                        "`{id}` cites `{}#{}`, which no persisted Evidence document carries",
                        claim.source, claim.id
                    ),
                ));
            }
        }
    }
    Ok(state)
}
