//! The propose reconciliation judgment leg.
//!
//! One schema-gated `create` over the same request envelope
//! `plan propose --dry-run` emits, with the same deterministic tail
//! `plan propose --from` runs (raw-bytes schema gate, typed parse) plus
//! a caller-supplied kernel check, inside the shared repair loop. The
//! caller owns the surrounding IO: assembling the request, running
//! `Plan::propose_from` for real under the atomic write loop, and the
//! journal bracket.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use specify_error::Error;
use specify_guest_model::Model;

use super::{prose, schema_gated};
use crate::change::{ProposalRequest, ProposalResponse, SourceBinding};
use crate::schema::validate_proposal_json;

/// Plan-authoring context for the Gate 1 prose the answer schema
/// requires.
///
/// Carries the plan name and its `plan.yaml.sources` bindings, so the
/// model can author the `discovery.md` source-inventory rows without
/// widening the pinned [`ProposalRequest`] envelope. Rendered as a
/// `## Plan context` section appended to the user message.
#[derive(Debug, Clone, Copy)]
pub struct GateContext<'a> {
    /// The plan name (`plan.yaml.name`).
    pub plan: &'a str,
    /// The plan's source bindings (`plan.yaml.sources`).
    pub sources: &'a BTreeMap<String, SourceBinding>,
}

impl GateContext<'_> {
    fn render(&self) -> String {
        let mut out = format!("## Plan context\n\n- plan: {}\n- sources:\n", self.plan);
        for (key, binding) in self.sources {
            let bound = match (&binding.path, &binding.value) {
                (Some(path), _) => format!("path `{path}`"),
                (None, Some(value)) => format!("value \"{value}\""),
                (None, None) => "no binding".to_string(),
            };
            // Writing to a String never fails.
            let _ = writeln!(out, "  - {key}: adapter `{}`, {bound}", binding.adapter);
        }
        out
    }
}

/// Run the lead-reconciliation judgment leg over an assembled request.
///
/// `check` is the kernel-projection dry run — typically
/// `Plan::propose_from` against a throwaway clone — so a grouping the
/// kernel would reject (coverage gap, source collision, unknown
/// project) is repaired in-loop rather than surfacing after the call.
///
/// `gate` is the optional plan-authoring context: when set, a `## Plan
/// context` section rides the user message so the model can author the
/// Gate 1 prose (`gate` on the answer) the collapsed `plan author`
/// orchestration persists.
///
/// # Errors
///
/// The mapped model failure, or the last schema / parse / check failure
/// once the repair budget is exhausted.
pub async fn reconcile<P, F>(
    model: &P, request: &ProposalRequest, gate: Option<GateContext<'_>>, mut check: F,
) -> Result<ProposalResponse, Error>
where
    P: Model,
    F: FnMut(&ProposalResponse) -> Result<(), Error>,
{
    let schema = specify_schema::answers::render(&specify_schema::answers::proposal());
    let mut user = format!(
        "## Reconciliation request\n\n```json\n{}\n```",
        super::render_json(request, "reconciliation request")?
    );
    if let Some(context) = gate {
        user.push_str("\n\n");
        user.push_str(&context.render());
    }
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
