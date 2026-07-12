use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{
    AssertionId, AssertionResult, HardAssertion, Outcome, OutputKind, Probe, Scenario, Stream,
};

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

/// Evidence available to deterministic grading: the trial workspace
/// root plus every captured step in execution order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Execution {
    root: PathBuf,
    steps: Vec<(String, StepResult)>,
}

impl Execution {
    /// Build captured execution evidence rooted at `root`. Steps keep
    /// the order they were captured in.
    #[must_use]
    pub fn new(
        root: impl Into<PathBuf>, steps: impl IntoIterator<Item = (String, StepResult)>,
    ) -> Self {
        Self {
            root: root.into(),
            steps: steps.into_iter().collect(),
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
        self.steps.iter().find(|(id, _)| id == step).map(|(_, result)| result)
    }

    /// Every captured step in execution order.
    pub fn steps(&self) -> impl Iterator<Item = (&str, &StepResult)> {
        self.steps.iter().map(|(id, result)| (id.as_str(), result))
    }
}

/// Verdict returned by a registered-probe evaluator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verdict {
    /// Whether the assertion held.
    pub passed: bool,
    /// Evidence path or compact probe output.
    pub evidence: String,
    /// Failure detail, when the assertion did not hold.
    pub detail: Option<String>,
}

impl Verdict {
    /// A passing verdict backed by `evidence`.
    #[must_use]
    pub fn pass(evidence: impl Into<String>) -> Self {
        Self {
            passed: true,
            evidence: evidence.into(),
            detail: None,
        }
    }

    /// A failing verdict backed by `evidence`, explained by `detail`.
    #[must_use]
    pub fn fail(evidence: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            passed: false,
            evidence: evidence.into(),
            detail: Some(detail.into()),
        }
    }
}

type Evaluator = Box<dyn Fn(&Execution) -> Verdict + Send + Sync>;

/// Profile-registered evaluators for `kind: registered` probes.
///
/// The registry type lives here so every harness settles registered
/// assertions through one grading entrypoint; the evaluator
/// implementations are registered by the owning harness (pure
/// filesystem evaluators ship in [`crate::evaluate`]; anything that
/// spawns a process stays harness-side).
#[derive(Default)]
pub struct Evaluators {
    registered: BTreeMap<AssertionId, Evaluator>,
}

impl Evaluators {
    /// Register `evaluator` for the assertion `id`.
    #[must_use]
    pub fn with(
        mut self, id: AssertionId,
        evaluator: impl Fn(&Execution) -> Verdict + Send + Sync + 'static,
    ) -> Self {
        self.registered.insert(id, Box::new(evaluator));
        self
    }
}

impl std::fmt::Debug for Evaluators {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Evaluators")
            .field("registered", &self.registered.keys().collect::<Vec<_>>())
            .finish()
    }
}

/// Evaluate every hard assertion in declaration order with no
/// registered evaluators: `kind: registered` probes fail with a detail
/// naming the missing profile-specific evaluator.
#[must_use]
pub fn hard(scenario: &Scenario, execution: &Execution) -> Vec<AssertionResult> {
    hard_with(scenario, execution, &Evaluators::default())
}

/// Evaluate every hard assertion in declaration order, settling
/// `kind: registered` probes through `evaluators`.
#[must_use]
pub fn hard_with(
    scenario: &Scenario, execution: &Execution, evaluators: &Evaluators,
) -> Vec<AssertionResult> {
    scenario
        .hard_assertions
        .iter()
        .map(|assertion| evaluate(assertion, execution, evaluators))
        .collect()
}

fn evaluate(
    assertion: &HardAssertion, execution: &Execution, evaluators: &Evaluators,
) -> AssertionResult {
    if matches!(&assertion.probe, Probe::Registered) {
        return evaluators.registered.get(&assertion.id).map_or_else(
            || AssertionResult {
                id: assertion.id,
                outcome: Outcome::Fail,
                evidence: None,
                detail: Some(format!(
                    "assertion `{}` requires a profile-specific evaluator",
                    assertion.id
                )),
            },
            |evaluator| {
                let verdict = evaluator(execution);
                AssertionResult {
                    id: assertion.id,
                    outcome: if verdict.passed { Outcome::Pass } else { Outcome::Fail },
                    evidence: Some(verdict.evidence),
                    detail: verdict.detail,
                }
            },
        );
    }

    let verdict = match &assertion.probe {
        Probe::Registered => unreachable!("registered probes are settled above"),
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

/// Check the scenario's declared `expected-outputs` against the trial
/// workspace, returning one detail line per missing or mismatched
/// output. Empty means every expected output is present.
#[must_use]
pub fn missing_outputs(scenario: &Scenario, execution: &Execution) -> Vec<String> {
    scenario
        .expected_outputs
        .iter()
        .filter_map(|output| {
            let path = execution.root.join(&output.path);
            let held = match output.kind {
                OutputKind::File => path.is_file(),
                OutputKind::Directory => path.is_dir(),
            };
            (!held).then(|| {
                format!(
                    "expected {} `{}` is absent",
                    match output.kind {
                        OutputKind::File => "file",
                        OutputKind::Directory => "directory",
                    },
                    output.path.display()
                )
            })
        })
        .collect()
}
