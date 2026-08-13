//! Bound delivery topology rows shared by `plan.yaml` and `discovery.yaml`.

use serde::{Deserialize, Serialize};

use crate::adapter::catalog::Pin;
use crate::snapshot::SnapshotId;

/// Reviewed-handoff identity copied onto `discovery.yaml` and `plan.yaml`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct DefinitionIdentity {
    /// Definition id the wave was projected from.
    pub system: String,
    /// Canonical handoff digest.
    pub handoff_digest: SnapshotId,
    /// Matching `system.wave.reviewed` identity.
    pub review: ReviewIdentity,
    /// Canonical digest of the system model.
    pub system_model_digest: SnapshotId,
    /// Canonical digest of the migration plan.
    pub migration_plan_digest: SnapshotId,
    /// Selected wave id.
    pub wave_id: String,
}

/// `(writer, sequence, event-digest)` of the imported review fact.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ReviewIdentity {
    /// Journal writer that appended the review fact.
    pub writer: String,
    /// Per-writer sequence of that fact.
    pub sequence: u64,
    /// Canonical digest of the review envelope.
    pub event_digest: SnapshotId,
}

/// One `plan.yaml.targets` / `discovery.yaml.targets` row.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct TargetBinding {
    /// Exact target-adapter package pin.
    pub adapter: Pin,
    /// Exact Git locator (`url@revision`).
    pub locator: String,
    /// Tree identity of that commit, excluding `.git` and a nested change home.
    pub cid: SnapshotId,
    /// Bound profile identity. Present on `plan.yaml.targets`; absent
    /// on `discovery.yaml.targets`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_capability_profile: Option<ProfileRef>,
}

impl TargetBinding {
    /// A target row without a model-capability profile (discovery).
    #[must_use]
    pub fn new(adapter: Pin, locator: impl Into<String>, cid: SnapshotId) -> Self {
        Self {
            adapter,
            locator: locator.into(),
            cid,
            model_capability_profile: None,
        }
    }

    /// Copy this row with `profile` stamped on.
    #[must_use]
    pub fn with_profile(mut self, profile: ProfileRef) -> Self {
        self.model_capability_profile = Some(profile);
        self
    }
}

/// Closed profile identity copied onto a target row.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub struct ProfileRef {
    /// Profile id.
    pub id: String,
    /// Canonical digest of the closed profile body.
    pub digest: SnapshotId,
}
