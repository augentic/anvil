//! The propose reconciliation judgment leg.
//!
//! One schema-gated `create` over the same request envelope
//! `plan propose --dry-run` emits, with the same deterministic tail
//! `plan propose --from` runs (raw-bytes schema gate, typed parse) plus
//! a caller-supplied kernel check, inside the shared repair loop. The
//! caller owns the surrounding IO: assembling the request, running
//! `Plan::propose_from` for real under the atomic write loop, and the
//! journal bracket.

use specify_error::Error;
use specify_guest_model::Model;

use super::{prose, schema_gated};
use crate::change::{ProposalRequest, ProposalResponse};
use crate::schema::validate_proposal_json;

/// Run the lead-reconciliation judgment leg over an assembled request.
///
/// `check` is the kernel-projection dry run — typically
/// `Plan::propose_from` against a throwaway clone — so a grouping the
/// kernel would reject (coverage gap, source collision, unknown
/// project) is repaired in-loop rather than surfacing after the call.
///
/// # Errors
///
/// The mapped model failure, or the last schema / parse / check failure
/// once the repair budget is exhausted.
pub async fn reconcile<P, F>(
    model: &P, request: &ProposalRequest, mut check: F,
) -> Result<ProposalResponse, Error>
where
    P: Model,
    F: FnMut(&ProposalResponse) -> Result<(), Error>,
{
    let schema = specify_schema::answers::render(&specify_schema::answers::proposal());
    let user = format!(
        "## Reconciliation request\n\n```json\n{}\n```",
        super::render_json(request, "reconciliation request")?
    );
    schema_gated(model, prose::PROPOSE, user, "proposal", &schema, |answer| {
        validate_proposal_json(answer)?;
        let response: ProposalResponse = serde_json::from_str(answer).map_err(|err| {
            Error::validation_failed(
                "plan-propose-response-parse",
                "the reconciliation answer deserialises as a response envelope",
                format!("failed to parse response envelope: {err}"),
            )
        })?;
        check(&response)?;
        Ok(response)
    })
    .await
}
