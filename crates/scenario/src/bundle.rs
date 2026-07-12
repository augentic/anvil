//! Run-bundle layout and report-completeness validation.
//!
//! Every quality runner — the engine repo's orchestrator and the
//! adapters repo's native runner alike — persists live evidence in one
//! bundle shape so reports stay comparable across repositories:
//!
//! ```text
//! quality/runs/<run-id>/
//!   report.json                    # the ScenarioReport
//!   trials/<n>/
//!     workspace/                   # the trial project root
//!     driver.log                   # driver stderr transcript
//!     result.json                  # the TrialResult
//!     rubric-<assertion-id>.json   # one raw verdict per rubric
//! ```
//!
//! [`validate()`](crate::bundle::validate) is the report-completeness
//! gate: a passed report must
//! carry every declared hard assertion and semantic rubric exactly
//! once per trial, all passing.

use std::fs;
use std::path::{Path, PathBuf};

use error::{Error, Result};

use crate::{AssertionId, Grading, Outcome, Scenario, ScenarioReport, TrialResult};

/// One run bundle rooted at `quality/runs/<run-id>` (or an override).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bundle {
    root: PathBuf,
}

impl Bundle {
    /// A bundle rooted at `root` (not created yet; see
    /// [`Bundle::create_trial`]).
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Bundle root directory.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Top-level report path.
    #[must_use]
    pub fn report_path(&self) -> PathBuf {
        self.root.join("report.json")
    }

    /// Directory for one one-based trial.
    #[must_use]
    pub fn trial_dir(&self, trial: usize) -> PathBuf {
        self.root.join("trials").join(trial.to_string())
    }

    /// Trial project workspace.
    #[must_use]
    pub fn workspace(&self, trial: usize) -> PathBuf {
        self.trial_dir(trial).join("workspace")
    }

    /// Driver stderr transcript for one trial.
    #[must_use]
    pub fn driver_log(&self, trial: usize) -> PathBuf {
        self.trial_dir(trial).join("driver.log")
    }

    /// Raw grader verdict for one rubric of one trial.
    #[must_use]
    pub fn rubric_verdict(&self, trial: usize, rubric: AssertionId) -> PathBuf {
        self.trial_dir(trial).join(format!("rubric-{rubric}.json"))
    }

    /// Structured result for one trial.
    #[must_use]
    pub fn trial_result(&self, trial: usize) -> PathBuf {
        self.trial_dir(trial).join("result.json")
    }

    /// Create one trial directory (and the bundle skeleton above it).
    ///
    /// # Errors
    ///
    /// Returns filesystem errors.
    pub fn create_trial(&self, trial: usize) -> Result<PathBuf> {
        let dir = self.trial_dir(trial);
        fs::create_dir_all(&dir).map_err(|source| Error::Filesystem {
            op: "create",
            path: dir.clone(),
            source,
        })?;
        Ok(dir)
    }

    /// Persist one trial's structured result as pretty JSON.
    ///
    /// # Errors
    ///
    /// Returns serialisation or filesystem errors.
    pub fn write_trial_result(&self, result: &TrialResult) -> Result<PathBuf> {
        let path = self.trial_result(result.trial);
        write_json(&path, result)?;
        Ok(path)
    }

    /// Persist the run report as pretty JSON.
    ///
    /// # Errors
    ///
    /// Returns serialisation or filesystem errors.
    pub fn write_report(&self, report: &ScenarioReport) -> Result<PathBuf> {
        let path = self.report_path();
        write_json(&path, report)?;
        Ok(path)
    }
}

fn write_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<()> {
    let body = serde_json::to_string_pretty(value).map_err(|error| Error::Diag {
        code: "scenario-report-serialise",
        detail: error.to_string(),
    })?;
    fs::write(path, format!("{body}\n")).map_err(|source| Error::Filesystem {
        op: "write",
        path: path.to_owned(),
        source,
    })
}

/// Validate report completeness against the canonical scenario.
///
/// Every trial must grade each declared hard assertion exactly once
/// and — when its profile grades semantically — each declared rubric
/// exactly once. A `pass` report additionally requires every trial,
/// assertion, and rubric to pass.
///
/// # Errors
///
/// Returns `scenario-report-incomplete` naming the first violation.
pub fn validate(scenario: &Scenario, report: &ScenarioReport) -> Result<()> {
    let fail = |detail: String| Error::Diag {
        code: "scenario-report-incomplete",
        detail,
    };
    if report.scenario != scenario.id {
        return Err(fail(format!(
            "report grades `{}` but the scenario is `{}`",
            report.scenario, scenario.id
        )));
    }
    if report.trials.is_empty() {
        return Err(fail("report carries no trials".to_owned()));
    }
    for trial in &report.trials {
        let profile = scenario
            .profiles
            .iter()
            .find(|profile| profile.id == trial.profile)
            .ok_or_else(|| {
                fail(format!("trial {} names undeclared profile `{}`", trial.trial, trial.profile))
            })?;
        exactly_once(
            trial.trial,
            "hard assertion",
            scenario.hard_assertions.iter().map(|assertion| assertion.id),
            trial.hard_assertions.iter().map(|result| result.id),
        )?;
        if profile.grading == Grading::Semantic {
            exactly_once(
                trial.trial,
                "semantic rubric",
                scenario.semantic_rubrics.iter().map(|rubric| rubric.id),
                trial.semantic_rubrics.iter().map(|result| result.id),
            )?;
        }
    }
    if report.outcome == Outcome::Pass {
        let failed = report.trials.iter().find(|trial| {
            trial.outcome != Outcome::Pass
                || trial.hard_assertions.iter().any(|result| result.outcome != Outcome::Pass)
                || trial.semantic_rubrics.iter().any(|result| result.outcome != Outcome::Pass)
        });
        if let Some(trial) = failed {
            return Err(fail(format!(
                "report passes but trial {} carries a non-passing verdict",
                trial.trial
            )));
        }
    }
    Ok(())
}

fn exactly_once(
    trial: usize, kind: &str, declared: impl Iterator<Item = AssertionId>,
    graded: impl Iterator<Item = AssertionId>,
) -> Result<()> {
    let declared: Vec<AssertionId> = declared.collect();
    let graded: Vec<AssertionId> = graded.collect();
    for id in &declared {
        let count = graded.iter().filter(|graded| *graded == id).count();
        if count != 1 {
            return Err(Error::Diag {
                code: "scenario-report-incomplete",
                detail: format!("trial {trial} grades {kind} `{id}` {count} times, expected once"),
            });
        }
    }
    if let Some(extra) = graded.iter().find(|id| !declared.contains(id)) {
        return Err(Error::Diag {
            code: "scenario-report-incomplete",
            detail: format!("trial {trial} grades undeclared {kind} `{extra}`"),
        });
    }
    Ok(())
}
