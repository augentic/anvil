//! Evidence answers
//!
//! The one model call an adapter makes: [`evidence`] asks the extract
//! question as an [`Evidence`]-typed [`Question`] and returns the accepted
//! document. [`content_note`] is the prompt fragment that tells the model
//! what it was bound to.
//!
//! The schema steers the answer's shape but cannot express every rule a
//! claim must satisfy, so each candidate the backend proposes is run through
//! the contract's claim gate before it is accepted; a miss goes back to the
//! model as findings and the backend asks again. The engine re-runs the same
//! gate on receipt, but an adapter that checks in place rarely hands it
//! evidence to reject.

use emery_source::claims;
use omnia_guest::Model;
use omnia_guest::model::Question;

use crate::references;
use crate::types::{Context, Error, Evidence, SourceContent, SourceInput};

/// Asks the extract question and returns the accepted [`Evidence`].
///
/// # Errors
///
/// Returns [`Error::InvalidRequest`] for a request the host refuses, and
/// [`Error::Internal`] for a model failure or the last gate findings once
/// the backend's rounds are spent.
pub async fn evidence<P: Model>(
    model: &P, ctx: &Context<'_>, system: String, user: String,
) -> Result<Evidence, Error> {
    let mut question = Question::<Evidence>::new("evidence").system(system);
    if !ctx.docs.is_empty() {
        question = question.tools(references::tools());
    }
    if let Some(lend) = ctx.lend.clone() {
        question = question.workspace(lend);
    }

    question
        .ask(model, user, references::answering(ctx.docs), |evidence| {
            let findings = claims::findings(&evidence.claims);
            if findings.is_empty() { Ok(()) } else { Err(findings) }
        })
        .await
        .map_err(Error::from)
}

/// Describes the bound source to the model; `tree` names what a workspace
/// holds (for example `the documentation tree`).
#[must_use]
pub fn content_note(input: &SourceInput, tree: &str) -> String {
    match &input.content {
        SourceContent::Workspace(root) => format!(
            "`$SOURCE_DIR` is the read-only view at `{root}` — {tree} the prompt walks. \
             Nothing outside it is reachable; extract mines only this source."
        ),
        SourceContent::Value(value) => format!(
            "The bound material is this inline value; no `$SOURCE_DIR` is lent:\n\n{value}\n\n\
             Nothing else is reachable; extract mines only this source."
        ),
    }
}
