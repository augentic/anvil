//! One trial's grading loop over a completed execution.
//!
//! Driving is profile-specific and stays with the binary; everything
//! after the workflow completes — hard assertions through the shared
//! registry, semantic rubrics through the [`Judge`], the declared
//! `expected-outputs` presence check, and per-trial bundle writes — is
//! one profile-agnostic pass that a credential-free test can exercise
//! with a fake judge.

use std::fs;

use anyhow::{Context as _, Result};
use scenario::bundle::Bundle;
use scenario::evaluate::semantic::{Judge, Rubrics};
use scenario::grade::{Evaluators, Execution};
use scenario::{Grading, Outcome, Profile, Scenario, TrialMetrics, TrialResult};

/// The run-constant grading inputs shared by every trial.
#[derive(Debug)]
pub struct Setting<'a> {
    /// The canonical scenario under evaluation.
    pub scenario: &'a Scenario,
    /// The live profile being trialled.
    pub profile: &'a Profile,
    /// Registered-probe evaluators for this profile.
    pub evaluators: &'a Evaluators,
    /// The shared rubric catalog.
    pub rubrics: &'a Rubrics,
    /// The run bundle receiving per-trial artifacts.
    pub bundle: &'a Bundle,
}

/// Grade one completed execution and persist the per-trial artifacts.
///
/// # Errors
///
/// Returns bundle-write failures only; assertion and rubric failures
/// are data on the returned [`TrialResult`].
pub async fn grade(
    setting: &Setting<'_>, execution: &Execution, judge: &impl Judge, trial: usize,
    duration_ms: usize,
) -> Result<TrialResult> {
    let hard_assertions =
        scenario::grade::hard_with(setting.scenario, execution, setting.evaluators);

    let mut semantic_rubrics = Vec::new();
    let mut outputs: Vec<std::path::PathBuf> = vec!["driver.log".into()];
    if setting.profile.grading == Grading::Semantic {
        for rubric in &setting.scenario.semantic_rubrics {
            let graded = scenario::evaluate::semantic::grade(
                rubric,
                setting.rubrics,
                execution.root(),
                judge,
            )
            .await;
            let verdict = setting.bundle.rubric_verdict(trial, rubric.id);
            fs::write(&verdict, &graded.raw).context("writing the rubric verdict")?;
            outputs.push(verdict.file_name().map(Into::into).unwrap_or_default());
            semantic_rubrics.push(graded.result);
        }
    }

    let missing_outputs = scenario::grade::missing_outputs(setting.scenario, execution);
    let passed = hard_assertions.iter().all(|result| result.outcome == Outcome::Pass)
        && semantic_rubrics.iter().all(|result| result.outcome == Outcome::Pass)
        && missing_outputs.is_empty();
    let result = TrialResult {
        trial,
        profile: setting.profile.id.clone(),
        outcome: if passed { Outcome::Pass } else { Outcome::Fail },
        hard_assertions,
        semantic_rubrics,
        missing_outputs,
        metrics: TrialMetrics {
            // The cursor backend exposes no token usage yet; keep the
            // counters stubbed and flagged unavailable.
            usage_available: false,
            input_tokens: 0,
            output_tokens: 0,
            reasoning_tokens: 0,
            duration_ms,
        },
        outputs,
    };
    setting.bundle.write_trial_result(&result).context("writing the trial result")?;
    Ok(result)
}
