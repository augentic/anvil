//! `plan correct` — record one durable operator correction.

use std::io::Write;

use omnia_guest::Model;
use omnia_guest::api::invoke::CallContext;
use omnia_guest::api::operation::Operation;
use project::adapter::Resolver;
use project::handler::{Anchor, Ctx, Render};
use project::journal::CorrectionConstraint;
use project::profile::Profiles;
use project::seam::{Source, Workspaces};
use serde::{Deserialize, Serialize};

use super::require_file;
use crate::orchestrate::{self, CorrectOutcome, CorrectionInput};

/// Wire input for `plan correct`.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CorrectInput {
    /// Domain id (or leaf slice name on an authored plan). Absent
    /// resolves the sole parked domain.
    #[serde(default)]
    pub domain: Option<String>,
    /// Closed structural constraint: `close-as-leaf` or `split`.
    #[serde(default)]
    pub constraint: Option<String>,
    /// Child domain ids a `split` constraint requires (repeatable).
    #[serde(default)]
    pub child: Vec<String>,
    /// Operator intent, verbatim.
    pub intent: String,
}

/// `emery plan correct [--domain <id>] [--constraint <c>] [--child
/// <id>]... --intent "…"`.
#[derive(Clone, Copy, Debug)]
pub struct Correct;

impl<P: Anchor + Model + Profiles + Resolver + Source + Workspaces> Operation<P> for Correct {
    type Error = project::handler::Error;
    type Input = CorrectInput;
    type Output = CorrectBody;

    async fn call(
        input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let cx = Ctx::load(context.provider)?;
        require_file(&cx)?;
        let correction = CorrectionInput {
            domain: input.domain,
            constraint: input.constraint.as_deref().map(parse_constraint).transpose()?,
            children: input.child,
            intent: input.intent,
        };
        let outcome =
            orchestrate::correct(context.provider, &cx.paths, cx.now(), correction).await?;
        Ok(outcome.into())
    }
}

fn parse_constraint(text: &str) -> Result<CorrectionConstraint, error::Error> {
    match text {
        "close-as-leaf" => Ok(CorrectionConstraint::CloseAsLeaf),
        "split" => Ok(CorrectionConstraint::Split),
        other => Err(error::Error::Argument {
            flag: "constraint",
            detail: format!("unknown constraint `{other}`; use `close-as-leaf` or `split`"),
        }),
    }
}

/// Success envelope for `plan correct`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct CorrectBody {
    /// `recorded` (fact only) or `proposed` (fact + boundary proposal).
    pub status: &'static str,
    /// Corrected domain id.
    pub domain: String,
    /// Retained proposal digest on the authored path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proposal: Option<String>,
}

impl From<CorrectOutcome> for CorrectBody {
    fn from(outcome: CorrectOutcome) -> Self {
        match outcome {
            CorrectOutcome::Recorded { domain } => Self {
                status: "recorded",
                domain,
                proposal: None,
            },
            CorrectOutcome::Proposed { domain, proposal } => Self {
                status: "proposed",
                domain,
                proposal: Some(proposal.to_string()),
            },
        }
    }
}

impl Render for CorrectBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        writeln!(w, "correction {} for domain `{}`", self.status, self.domain)?;
        match &self.proposal {
            Some(digest) => writeln!(
                w,
                "apply it with emery plan amend --proposal {digest}, or discard by leaving it \
                 unapplied"
            ),
            None => writeln!(w, "re-run emery plan author — re-entry honors the correction"),
        }
    }
}
