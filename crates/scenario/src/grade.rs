use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{AssertionResult, HardAssertion, Outcome, Probe, Scenario, Stream};

/// Captured result of one workflow step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct StepResult {
    /// Process exit code, or `-1` when no process status exists.
    pub exit_code: i32,
    /// Captured standard output.
    pub stdout: String,
    /// Captured standard error.
    pub stderr: String,
}

/// Evidence available to deterministic grading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Execution {
    root: PathBuf,
    steps: BTreeMap<String, StepResult>,
}

impl Execution {
    /// Build captured execution evidence rooted at `root`.
    #[must_use]
    pub fn new(root: impl Into<PathBuf>, steps: BTreeMap<String, StepResult>) -> Self {
        Self {
            root: root.into(),
            steps,
        }
    }

    /// Trial workspace inspected by filesystem probes.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Captured result for `step`.
    #[must_use]
    pub fn step(&self, step: &str) -> Option<&StepResult> {
        self.steps.get(step)
    }
}

/// Evaluate every hard assertion in declaration order.
#[must_use]
pub fn hard(scenario: &Scenario, execution: &Execution) -> Vec<AssertionResult> {
    scenario.hard_assertions.iter().map(|assertion| evaluate(assertion, execution)).collect()
}

fn evaluate(assertion: &HardAssertion, execution: &Execution) -> AssertionResult {
    let verdict = match &assertion.probe {
        Probe::Registered => {
            Err(format!("assertion `{}` requires a profile-specific evaluator", assertion.id))
        }
        Probe::ExitCode { step, equals } => execution.step(step).map_or_else(
            || Err(format!("workflow step `{step}` has no captured result")),
            |result| compare(result.exit_code == *equals, format!("exit={}", result.exit_code)),
        ),
        Probe::PathExists { path } => compare(
            execution.root.join(path).exists(),
            execution.root.join(path).display().to_string(),
        ),
        Probe::PathAbsent { path } => compare(
            !execution.root.join(path).exists(),
            execution.root.join(path).display().to_string(),
        ),
        Probe::StreamContains { step, stream, value } => execution.step(step).map_or_else(
            || Err(format!("workflow step `{step}` has no captured result")),
            |result| {
                let body = match stream {
                    Stream::Stdout => &result.stdout,
                    Stream::Stderr => &result.stderr,
                };
                compare(body.contains(value), body.clone())
            },
        ),
        Probe::JsonEquals { step, pointer, value } => execution.step(step).map_or_else(
            || Err(format!("workflow step `{step}` has no captured result")),
            |result| {
                serde_json::from_str::<serde_json::Value>(&result.stdout)
                    .map_err(|error| format!("step `{step}` stdout is not JSON: {error}"))
                    .and_then(|document| {
                        let actual = document
                            .pointer(pointer)
                            .ok_or_else(|| format!("JSON pointer `{pointer}` is absent"))?;
                        compare(actual == value, actual.to_string())
                    })
            },
        ),
    };

    match verdict {
        Ok(evidence) => AssertionResult {
            id: assertion.id,
            outcome: Outcome::Pass,
            evidence: Some(evidence),
            detail: None,
        },
        Err(detail) => AssertionResult {
            id: assertion.id,
            outcome: Outcome::Fail,
            evidence: None,
            detail: Some(detail),
        },
    }
}

fn compare(passed: bool, evidence: String) -> Result<String, String> {
    if passed { Ok(evidence) } else { Err(format!("probe did not match; observed `{evidence}`")) }
}
