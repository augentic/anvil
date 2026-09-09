//! Briefs
//!
//! One judgment put to the model: what the engine asks, how it steers the
//! answer, and what it accepts. A brief carries a run's facts, names the
//! prose that instructs the model, renders the turn, tightens the answer's
//! derived schema to the run, and holds every candidate to the facts; only
//! an answer its check accepts is concluded — into rows or a document — and
//! the brief alone does the concluding.
//!
//! The model is never asked for anything the engine can decide itself, and
//! nothing the engine renders comes from an unchecked answer.

use std::fmt;

use omnia_guest::model::{Findings, Question};
use omnia_guest::{Error, Model};
use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde_json::Value;

// `Sync`: the check closure `Question::ask` takes is `Send`, and it
// borrows the brief.
pub trait Brief: fmt::Display + Sync + Sized {
    /// The typed answer the brief asks for.
    type Answer: JsonSchema + DeserializeOwned + Send;

    /// What the judgment yields: rows, a document.
    type Output;

    /// The question's name.
    const NAME: &'static str;

    /// The synthesis prose, in prompt order.
    const PROSE: &'static [&'static str];

    /// Tightens the derived `schema` toward this run. Hints for the
    /// provider; [`Self::check`] is the gate.
    fn hints(&self, schema: &mut Value);

    /// Holds a candidate answer to the run's facts.
    ///
    /// # Errors
    ///
    /// Returns every finding, for repair.
    fn check(&self, answer: &Self::Answer) -> Result<(), Findings>;

    /// Derives the output from the brief and the answer its check accepted.
    fn conclude(self, answer: Self::Answer) -> Self::Output;

    /// Puts the brief to `model` and concludes the answer its check
    /// accepted.
    ///
    /// # Errors
    ///
    /// A model failure is `bad_gateway`; a candidate outside the answer's
    /// shape or the backend's spent rounds is `bad_request`.
    async fn judge<M: Model>(self, model: &M) -> Result<Self::Output, Error> {
        tracing::info!(question = Self::NAME, "asking the model");
        let system = Self::PROSE
            .iter()
            .map(|path| crate::prose::body(path))
            .collect::<Vec<_>>()
            .join("\n\n---\n\n");
        let answer = Question::<Self::Answer>::new(Self::NAME)
            .system(system)
            .schema(|schema| self.hints(schema))
            .ask(model, self.to_string(), None, |answer| self.check(answer))
            .await?;

        Ok(self.conclude(answer))
    }
}

// `Ok(())` for no findings, else every finding for repair.
pub fn verdict(findings: Findings) -> Result<(), Findings> {
    if findings.is_empty() { Ok(()) } else { Err(findings) }
}
