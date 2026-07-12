//! Semantic rubric grading behind the [`Judge`] seam.
//!
//! Rubric criteria live in two places: the canonical scenario carries
//! each rubric's criterion sentence, and the shared rubric catalog
//! (`quality/rubrics/semantic.yaml`) carries the grading scale and the
//! canonical question per assertion id. This module owns the prompt,
//! the verdict validation, and the catalog loading; producing a raw
//! verdict is the owning harness's [`Judge`] implementation (the live
//! one runs on the omnia model seam) — this crate spawns nothing.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use error::{Error, Result};
use serde::Deserialize;

use crate::{Outcome, RubricResult, SemanticRubric};

/// A semantic grader: turns one rubric prompt into a raw JSON verdict.
///
/// Implementations own the model transport (live cursor backend,
/// scripted fake, …). The verdict contract is one JSON object with
/// keys `score` (integer on the catalog scale), `outcome` (`pass` or
/// `fail`), and `detail` (evidence-based explanation);
/// [`grade`] validates it either way, so a judge may return its
/// transport's raw output unchecked. An `Err` is a judge that could
/// not run at all and grades as [`Outcome::Error`].
///
/// `Sync` is a supertrait so grading futures stay `Send` for
/// multi-threaded runtimes.
pub trait Judge: Sync {
    /// Produce the raw verdict for `prompt`, judged from `workspace`.
    fn judge(
        &self, prompt: String, workspace: &Path,
    ) -> impl Future<Output = std::result::Result<String, String>> + Send;
}

/// The shared rubric catalog: grading scale plus one question per
/// assertion id.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Rubrics {
    /// Catalog schema version.
    pub version: u32,
    /// Score range and thresholds.
    pub scale: Scale,
    /// Canonical questions keyed by assertion id.
    pub criteria: BTreeMap<String, Criterion>,
}

/// Score range and pass/review thresholds.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Scale {
    /// Lowest score a grader may assign.
    pub minimum: u8,
    /// Highest score a grader may assign.
    pub maximum: u8,
    /// Scores at or above this value pass.
    pub pass: u8,
    /// Scores below this value request operator review.
    pub review_below: u8,
}

/// One catalog criterion.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Criterion {
    /// The question a grader applies.
    pub question: String,
}

impl Rubrics {
    /// Load the rubric catalog from `path`.
    ///
    /// # Errors
    ///
    /// Returns filesystem or YAML errors.
    pub fn load(path: &Path) -> Result<Self> {
        let input = fs::read_to_string(path).map_err(|source| Error::Filesystem {
            op: "read",
            path: path.to_owned(),
            source,
        })?;
        Ok(serde_saphyr::from_str(&input)?)
    }
}

/// One completed semantic grade: the structured verdict plus the raw
/// judge output for the evidence bundle.
#[derive(Debug, Clone)]
pub struct Graded {
    /// The validated verdict.
    pub result: RubricResult,
    /// Raw judge output (the verdict JSON on success).
    pub raw: String,
}

/// Grade one rubric through `judge` from the trial workspace. A judge
/// that cannot run or returns a malformed verdict is an
/// [`Outcome::Error`], never a silent pass.
pub async fn grade(
    rubric: &SemanticRubric, rubrics: &Rubrics, workspace: &Path, judge: &impl Judge,
) -> Graded {
    let prompt = prompt(rubric, rubrics);
    let raw = match judge.judge(prompt, workspace).await {
        Ok(raw) => raw,
        Err(detail) => {
            return Graded {
                result: error_result(rubric, format!("semantic judge could not run: {detail}")),
                raw: String::new(),
            };
        }
    };
    let result = parse_verdict(&raw, rubrics.scale.pass).map_or_else(
        |detail| error_result(rubric, detail),
        |(score, detail)| RubricResult {
            id: rubric.id,
            outcome: if score >= rubrics.scale.pass { Outcome::Pass } else { Outcome::Fail },
            score: Some(score),
            evidence: format!("rubric-{}.json", rubric.id),
            detail: Some(detail),
        },
    );
    Graded { result, raw }
}

/// Build the one-shot grading prompt from the scenario rubric and the
/// catalog question.
fn prompt(rubric: &SemanticRubric, rubrics: &Rubrics) -> String {
    let evidence = rubric
        .evidence
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let question = rubrics
        .criteria
        .get(&rubric.id.to_string())
        .map_or_else(|| rubric.criterion.clone(), |criterion| criterion.question.clone());
    format!(
        "Read the evidence in this workspace (start with: {evidence}). Grade only the \
         `{id}` criterion: {question} Criterion statement: {criterion} Return exactly one \
         compact JSON object with keys score (integer {min}-{max}), outcome (pass or fail; \
         pass requires score >= {pass}), and detail (concise evidence-based explanation).",
        id = rubric.id,
        criterion = rubric.criterion,
        min = rubrics.scale.minimum,
        max = rubrics.scale.maximum,
        pass = rubrics.scale.pass,
    )
}

/// Validate the judge's JSON verdict, returning `(score, detail)`.
fn parse_verdict(raw: &str, pass: u8) -> std::result::Result<(u8, String), String> {
    let value: serde_json::Value = serde_json::from_str(raw.trim())
        .map_err(|error| format!("semantic judge did not return valid JSON: {error}"))?;
    let score = value
        .get("score")
        .and_then(serde_json::Value::as_u64)
        .filter(|score| *score <= 100)
        .ok_or_else(|| "semantic judge returned no integer score in 0-100".to_owned())?;
    let outcome = value.get("outcome").and_then(serde_json::Value::as_str);
    if !matches!(outcome, Some("pass" | "fail")) {
        return Err("semantic judge returned no pass/fail outcome".to_owned());
    }
    let detail = value
        .get("detail")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "semantic judge returned no detail string".to_owned())?;
    let score = u8::try_from(score).expect("score is bounded by 100");
    let claimed_pass = outcome == Some("pass");
    let detail = if claimed_pass == (score >= pass) {
        detail.to_owned()
    } else {
        format!("{detail} (judge outcome disagreed with its score; the score decides)")
    };
    Ok((score, detail))
}

/// A grade that could not complete.
fn error_result(rubric: &SemanticRubric, detail: String) -> RubricResult {
    RubricResult {
        id: rubric.id,
        outcome: Outcome::Error,
        score: None,
        evidence: format!("rubric-{}.json", rubric.id),
        detail: Some(detail),
    }
}
