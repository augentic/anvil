//! The propose reconciliation judgment leg: one schema-gated `create`
//! over the proposal request envelope inside the shared repair loop.
//! The caller owns the surrounding IO and the journal bracket.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use error::Error;
use omnia_guest::Model;
use project::judgment::{render_json, repaired};
use project::plan::{ProposalRequest, ProposalResponse, SourceBinding};

use crate::judgment::prose;

/// Plan-authoring context for the review prose the answer schema
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
/// `check` is the kernel-projection dry run (typically
/// `Plan::propose_from` against a throwaway clone), so a grouping the
/// kernel would reject is repaired in-loop rather than after the call.
/// `gate`, when set, rides a `## Plan context` section on the user
/// message so the model can author the persisted review prose.
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
    let schema = project::answers::render(&project::answers::proposal());
    let mut user = format!(
        "## Reconciliation request\n\n```json\n{}\n```",
        render_json(request, "reconciliation request")?
    );

    if let Some(context) = gate {
        user.push_str("\n\n");
        user.push_str(&context.render());
    }

    repaired(model, prose::propose(), user, "proposal", &schema, |answer| {
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
