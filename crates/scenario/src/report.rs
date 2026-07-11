use std::collections::BTreeMap;
use std::path::PathBuf;

use jiff::Timestamp;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::AssertionId;

/// Type-level pin of the structured scenario report version.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ScenarioReportVersion;

impl Serialize for ScenarioReportVersion {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_u32(1)
    }
}

impl<'de> Deserialize<'de> for ScenarioReportVersion {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = u32::deserialize(deserializer)?;
        if value == 1 {
            Ok(Self)
        } else {
            Err(serde::de::Error::custom(format!(
                "unsupported scenario report version: {value} (only v1 is supported)"
            )))
        }
    }
}

/// Outcome shared by reports, trials, assertions, and rubrics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Outcome {
    /// Evaluation passed.
    Pass,
    /// Evaluation completed and failed.
    Fail,
    /// Evaluation could not complete.
    Error,
    /// Evaluation was intentionally not run.
    Skipped,
}

/// Immutable metadata describing one scenario run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct RunMetadata {
    /// Unique run identifier assigned by the owning harness.
    pub id: String,
    /// Runner implementation name and version.
    pub runner: String,
    /// Repository revisions under evaluation, keyed by repository.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub revisions: BTreeMap<String, String>,
    /// Live model identity, when the profile uses one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Digest of the prompt/reference inputs offered to the model.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt_digest: Option<String>,
    /// Adapter or component digests used by the run.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub component_digests: BTreeMap<String, String>,
    /// Start time.
    pub started_at: Timestamp,
    /// Completion time.
    pub completed_at: Timestamp,
}

/// Verdict for one deterministic assertion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct AssertionResult {
    /// Stable assertion taxonomy id.
    pub id: AssertionId,
    /// Deterministic verdict.
    pub outcome: Outcome,
    /// Evidence path or compact probe output.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<String>,
    /// Failure or execution detail.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Verdict for one semantic rubric.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct RubricResult {
    /// Stable assertion taxonomy id.
    pub id: AssertionId,
    /// Evidence-backed verdict.
    pub outcome: Outcome,
    /// Optional score in the inclusive `0..=100` range.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score: Option<u8>,
    /// Required evidence pointer for a completed semantic grade.
    pub evidence: String,
    /// Grader explanation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Result of one independent profile trial.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TrialResult {
    /// One-based trial number within the profile.
    pub trial: usize,
    /// Profile id used for this trial.
    pub profile: String,
    /// Aggregate trial outcome.
    pub outcome: Outcome,
    /// Deterministic assertion results in scenario declaration order.
    pub hard_assertions: Vec<AssertionResult>,
    /// Semantic grades in scenario declaration order.
    pub semantic_rubrics: Vec<RubricResult>,
    /// Resource and timing measurements for this trial.
    #[serde(default)]
    pub metrics: TrialMetrics,
    /// Relative outputs captured or retained by the harness.
    #[serde(default)]
    pub outputs: Vec<PathBuf>,
}

/// Resource and timing measurements for one trial.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TrialMetrics {
    /// Whether token counters were exposed by the selected backend.
    pub usage_available: bool,
    /// Model input tokens consumed.
    pub input_tokens: usize,
    /// Model output tokens consumed.
    pub output_tokens: usize,
    /// Model reasoning tokens consumed.
    pub reasoning_tokens: usize,
    /// Wall-clock duration in milliseconds.
    pub duration_ms: usize,
}

/// Structured result envelope for a complete scenario run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ScenarioReport {
    /// Report schema version, currently `1`.
    pub version: ScenarioReportVersion,
    /// Canonical scenario id.
    pub scenario: String,
    /// Aggregate run outcome.
    pub outcome: Outcome,
    /// Run-level provenance and timing.
    pub run: RunMetadata,
    /// Trial results grouped in execution order.
    pub trials: Vec<TrialResult>,
}
