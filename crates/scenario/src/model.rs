use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

use error::{Error, Result};
use schema::{SCENARIO_JSON_SCHEMA, ValidationStatus, join_details, validate_value_cached};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

use crate::{AssertionId, AssertionKind};

/// Type-level pin of the canonical scenario document version.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ScenarioVersion;

impl Serialize for ScenarioVersion {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_u32(1)
    }
}

impl<'de> Deserialize<'de> for ScenarioVersion {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let value = u32::deserialize(deserializer)?;
        if value == 1 {
            Ok(Self)
        } else {
            Err(serde::de::Error::custom(format!(
                "unsupported scenario version: {value} (only v1 is supported)"
            )))
        }
    }
}

/// Frequency and blocking tier for a scenario.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GateTier {
    /// Must pass for every release.
    ReleaseBlocker,
    /// Runs with the complete periodic scenario sweep.
    Full,
}

/// Filesystem-state isolation contract for each trial.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Isolation {
    /// Start from a newly initialised project.
    FreshProject,
    /// Reuse a baseline while isolating trial mutations.
    SharedBaseline,
    /// Reuse an existing slice tree.
    SharedSlice,
}

/// Declarative preparation performed before the workflow begins.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Setup {
    /// Commands executed in declaration order by a future executor.
    #[serde(default)]
    pub commands: Vec<String>,
    /// Environment additions applied to setup and workflow steps.
    #[serde(default)]
    pub environment: BTreeMap<String, String>,
}

/// Kind of workflow interaction represented by a step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorkflowStepKind {
    /// Invoke a process through the owning harness.
    Command,
    /// Send an operator prompt through the owning harness.
    Prompt,
}

/// One ordered workflow interaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct WorkflowStep {
    /// Scenario-local kebab-case identifier.
    pub id: String,
    /// Interaction mechanism.
    pub kind: WorkflowStepKind,
    /// Command line or prompt text consumed by the owning harness.
    pub run: String,
    /// Optional execution profile id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    /// Fixture ids materialised before this step.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fixtures: Vec<String>,
}

/// Named runtime, model, and grading selection for repeated trials.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Profile {
    /// Scenario-local kebab-case identifier.
    pub id: String,
    /// Execution environment used for every trial.
    pub runtime: Runtime,
    /// Judgment backend used for every trial.
    pub model: ModelBackend,
    /// Grading applied after hard assertions complete.
    pub grading: Grading,
    /// Number of independent trials.
    pub trials: usize,
    /// Environment additions specific to this profile.
    #[serde(default)]
    pub environment: BTreeMap<String, String>,
}

/// Runtime selected by an execution profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Runtime {
    /// Linked Rust adapter crates through `specify-adapters/harness/native`.
    Native,
    /// Hosted workflow and adapter WebAssembly components.
    Wasm,
}

/// Model backend selected by an execution profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ModelBackend {
    /// Ordered responses declared by a deterministic test.
    Scripted,
    /// Canonical request-key fixtures through `omnia-testkit`.
    Replay,
    /// Live model completions.
    Live,
}

/// Grading mode selected by an execution profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Grading {
    /// Evaluate only deterministic assertions.
    Hard,
    /// Evaluate deterministic assertions and semantic rubrics.
    Semantic,
}

/// Input tree copied into a trial workspace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Fixture {
    /// Scenario-local kebab-case identifier.
    pub id: String,
    /// Source path relative to the scenario document.
    pub source: PathBuf,
    /// Destination path relative to the trial workspace.
    pub destination: PathBuf,
}

/// Deterministic probe shape evaluated after a workflow step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Probe {
    /// Runtime-specific evaluator registered for the assertion id.
    Registered,
    /// A command exits with the expected status code.
    ExitCode {
        /// Step whose process result is inspected.
        step: String,
        /// Required process status.
        equals: i32,
    },
    /// A relative path exists.
    PathExists {
        /// Path relative to the trial workspace.
        path: PathBuf,
    },
    /// A relative path does not exist.
    PathAbsent {
        /// Path relative to the trial workspace.
        path: PathBuf,
    },
    /// A process stream contains a literal fragment.
    StreamContains {
        /// Step whose process result is inspected.
        step: String,
        /// Stream to inspect.
        stream: Stream,
        /// Required literal fragment.
        value: String,
    },
    /// A JSON pointer in a step's standard output equals a value.
    JsonEquals {
        /// Step whose standard output is parsed.
        step: String,
        /// RFC 6901 JSON pointer.
        pointer: String,
        /// Required JSON value.
        value: Value,
    },
}

/// Captured process stream inspected by a hard assertion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Stream {
    /// Standard output.
    Stdout,
    /// Standard error.
    Stderr,
}

/// One machine-verifiable assertion and its deterministic probe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct HardAssertion {
    /// Stable assertion taxonomy id.
    pub id: AssertionId,
    /// Step after which the probe is evaluated.
    pub after: String,
    /// Deterministic grading operation.
    pub probe: Probe,
}

/// One evidence-backed qualitative grading criterion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct SemanticRubric {
    /// Stable assertion taxonomy id.
    pub id: AssertionId,
    /// Step after which evidence is collected.
    pub after: String,
    /// Criterion a grader applies.
    pub criterion: String,
    /// Relative paths expected to contain grading evidence.
    #[serde(default)]
    pub evidence: Vec<PathBuf>,
}

/// Expected filesystem output kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OutputKind {
    /// Regular file.
    File,
    /// Directory tree.
    Directory,
}

/// Output a successful scenario is expected to leave behind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ExpectedOutput {
    /// Path relative to the trial workspace.
    pub path: PathBuf,
    /// Expected filesystem object kind.
    pub kind: OutputKind,
}

/// Canonical, executor-neutral YAML scenario.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct Scenario {
    /// Schema version, currently `1`.
    pub version: ScenarioVersion,
    /// Globally unique kebab-case identifier.
    pub id: String,
    /// Owning suite, plugin, or adapter.
    pub owner: String,
    /// Gate frequency and blocking policy.
    pub gate_tier: GateTier,
    /// Trial workspace isolation policy.
    pub isolation: Isolation,
    /// Declarative pre-workflow setup.
    pub setup: Setup,
    /// Ordered workflow interactions.
    pub workflow: Vec<WorkflowStep>,
    /// Named execution profiles.
    pub profiles: Vec<Profile>,
    /// Named fixture trees.
    pub fixtures: Vec<Fixture>,
    /// Deterministic assertions.
    pub hard_assertions: Vec<HardAssertion>,
    /// Evidence-backed qualitative criteria.
    pub semantic_rubrics: Vec<SemanticRubric>,
    /// Filesystem outputs expected after a successful run.
    pub expected_outputs: Vec<ExpectedOutput>,
}

impl Scenario {
    /// Parse and validate one canonical YAML document.
    ///
    /// # Errors
    ///
    /// Returns YAML, schema-validation, or cross-reference validation errors.
    pub fn from_yaml(input: &str) -> Result<Self> {
        let value: Value = serde_saphyr::from_str(input)?;
        validate_schema(&value)?;
        let scenario: Self = serde_saphyr::from_str(input)?;
        scenario.validate()?;
        Ok(scenario)
    }

    /// Load and validate one canonical YAML document from `path`.
    ///
    /// # Errors
    ///
    /// Returns a filesystem error when the document cannot be read, or the
    /// same validation errors as [`Self::from_yaml`].
    pub fn load(path: &Path) -> Result<Self> {
        let input = fs::read_to_string(path).map_err(|source| Error::Filesystem {
            op: "read",
            path: path.to_owned(),
            source,
        })?;
        Self::from_yaml(&input)
    }

    /// Validate schema shape, taxonomy use, references, and relative paths.
    ///
    /// # Errors
    ///
    /// Returns `scenario-schema` for schema failures or `scenario-contract`
    /// for invalid references and taxonomy assignments.
    pub fn validate(&self) -> Result<()> {
        let value = serde_json::to_value(self).map_err(|err| Error::Diag {
            code: "scenario-serialise",
            detail: format!("failed to serialise scenario for validation: {err}"),
        })?;
        validate_schema(&value)?;
        self.validate_contract()
    }

    fn validate_contract(&self) -> Result<()> {
        let steps = unique_ids("workflow", self.workflow.iter().map(|step| step.id.as_str()))?;
        let profiles = unique_ids("profiles", self.profiles.iter().map(|p| p.id.as_str()))?;
        let fixtures = unique_ids("fixtures", self.fixtures.iter().map(|f| f.id.as_str()))?;
        for profile in &self.profiles {
            if profile.trials == 0 {
                return contract(format!(
                    "profile `{}` must declare at least one trial",
                    profile.id
                ));
            }
            if profile.grading == Grading::Semantic && profile.model != ModelBackend::Live {
                return contract(format!(
                    "profile `{}` uses semantic grading without a live model",
                    profile.id
                ));
            }
            if profile.model == ModelBackend::Live && profile.grading != Grading::Semantic {
                return contract(format!(
                    "live profile `{}` must use semantic grading",
                    profile.id
                ));
            }
        }
        for step in &self.workflow {
            if let Some(profile) = &step.profile
                && !profiles.contains(profile.as_str())
            {
                return contract(format!(
                    "workflow step `{}` references unknown profile `{profile}`",
                    step.id
                ));
            }
            for fixture in &step.fixtures {
                if !fixtures.contains(fixture.as_str()) {
                    return contract(format!(
                        "workflow step `{}` references unknown fixture `{fixture}`",
                        step.id
                    ));
                }
            }
        }
        for fixture in &self.fixtures {
            safe_relative(&fixture.source, "fixture source")?;
            safe_relative(&fixture.destination, "fixture destination")?;
        }
        for assertion in &self.hard_assertions {
            require_step(&steps, &assertion.after, "hard assertion")?;
            if assertion.id.metadata().kind != AssertionKind::Hard {
                return contract(format!("assertion `{}` is semantic, not hard", assertion.id));
            }
            match &assertion.probe {
                Probe::Registered => {}
                Probe::ExitCode { step, .. }
                | Probe::StreamContains { step, .. }
                | Probe::JsonEquals { step, .. } => require_step(&steps, step, "probe")?,
                Probe::PathExists { path } | Probe::PathAbsent { path } => {
                    safe_relative(path, "probe path")?;
                }
            }
        }
        for rubric in &self.semantic_rubrics {
            require_step(&steps, &rubric.after, "semantic rubric")?;
            if rubric.id.metadata().kind != AssertionKind::Semantic {
                return contract(format!("assertion `{}` is hard, not semantic", rubric.id));
            }
            for evidence in &rubric.evidence {
                safe_relative(evidence, "rubric evidence")?;
            }
        }
        for output in &self.expected_outputs {
            safe_relative(&output.path, "expected output")?;
        }
        Ok(())
    }
}

fn validate_schema(value: &Value) -> Result<()> {
    let failures = validate_value_cached(
        value,
        SCENARIO_JSON_SCHEMA,
        "scenario-schema",
        "canonical scenario matches its JSON Schema",
    )
    .into_iter()
    .filter(|summary| summary.status == ValidationStatus::Fail)
    .collect::<Vec<_>>();
    if failures.is_empty() {
        Ok(())
    } else {
        Err(Error::validation_failed("scenario-schema", "", join_details(&failures)))
    }
}

fn unique_ids<'a>(
    collection: &str, ids: impl Iterator<Item = &'a str>,
) -> Result<BTreeSet<&'a str>> {
    let mut unique = BTreeSet::new();
    for id in ids {
        if !unique.insert(id) {
            return contract(format!("{collection} contains duplicate id `{id}`"));
        }
    }
    Ok(unique)
}

fn require_step(steps: &BTreeSet<&str>, step: &str, subject: &str) -> Result<()> {
    if steps.contains(step) {
        Ok(())
    } else {
        contract(format!("{subject} references unknown workflow step `{step}`"))
    }
}

fn safe_relative(path: &Path, subject: &str) -> Result<()> {
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path.components().any(|component| matches!(component, Component::ParentDir))
    {
        contract(format!("{subject} `{}` must be a non-escaping relative path", path.display()))
    } else {
        Ok(())
    }
}

fn contract<T>(detail: String) -> Result<T> {
    Err(Error::validation_failed(
        "scenario-contract",
        "canonical scenario references are coherent",
        detail,
    ))
}
