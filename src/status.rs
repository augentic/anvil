use std::fmt;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::pipeline::Pipeline;
use crate::registry::Registry;

/// Lifecycle state of a pipeline target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TargetState {
    /// Not yet distributed to repos.
    Pending,
    /// Changes distributed, branch/PR created.
    Distributed,
    /// Implementation in progress.
    Applying,
    /// Implementation complete, ready for review.
    Implemented,
    /// PR under review.
    Reviewing,
    /// PR merged.
    Merged,
    /// Failed at some step; can be retried.
    Failed,
}

impl TargetState {
    /// Ordinal for "at least this far" comparisons in the happy path.
    const fn ordinal(self) -> u8 {
        match self {
            Self::Pending | Self::Failed => 0,
            Self::Distributed => 1,
            Self::Applying => 2,
            Self::Implemented => 3,
            Self::Reviewing => 4,
            Self::Merged => 5,
        }
    }

    /// True if this state is at or past the threshold on the happy path (Failed is treated as behind).
    pub fn is_at_least(self, threshold: Self) -> bool {
        self != Self::Failed && self.ordinal() >= threshold.ordinal()
    }
}

impl fmt::Display for TargetState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Pending => "pending",
            Self::Distributed => "distributed",
            Self::Applying => "applying",
            Self::Implemented => "implemented",
            Self::Reviewing => "reviewing",
            Self::Merged => "merged",
            Self::Failed => "failed",
        };
        f.write_str(s)
    }
}

/// Per-target status within a pipeline run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetStatus {
    pub id: String,
    pub repo: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pr: Option<String>,
    pub state: TargetState,
}

/// Persisted status for a change across all pipeline targets.
#[derive(Debug, Serialize, Deserialize)]
pub struct PipelineStatus {
    pub change: String,
    pub updated: String,
    pub targets: Vec<TargetStatus>,
}

impl PipelineStatus {
    /// Load status from a TOML file.
    pub fn load(path: &Path) -> Result<Self> {
        crate::util::load_toml(path)
    }

    /// Load existing status or create a new one with all targets in `Pending`.
    pub fn load_or_create(
        path: &Path, change: &str, pipeline: &Pipeline, registry: &Registry,
    ) -> Result<Self> {
        if path.exists() {
            return Self::load(path);
        }

        let targets = pipeline
            .targets
            .iter()
            .map(|t| {
                let svc = registry
                    .find_by_id(&t.id)
                    .with_context(|| format!("target '{}' not found in registry", t.id))?;
                Ok(TargetStatus {
                    id: t.id.clone(),
                    repo: svc.repo.clone(),
                    pr: None,
                    state: TargetState::Pending,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            change: change.to_string(),
            updated: now(),
            targets,
        })
    }

    /// Persist status to a TOML file.
    pub fn save(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self).context("serializing status")?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content).with_context(|| format!("writing {}", path.display()))
    }

    /// Look up target status by ID.
    pub fn get(&self, id: &str) -> Option<&TargetStatus> {
        self.targets.iter().find(|t| t.id == id)
    }

    /// Move a target to a new state; validates allowed transitions.
    pub fn transition(&mut self, id: &str, new_state: TargetState) -> Result<()> {
        let target = self
            .targets
            .iter_mut()
            .find(|t| t.id == id)
            .with_context(|| format!("target '{id}' not in status"))?;

        let allowed = matches!(
            (target.state, new_state),
            // Forward transitions (grouped by destination)
            (
                TargetState::Pending | TargetState::Failed | TargetState::Distributed,
                TargetState::Distributed
            ) | (
                TargetState::Distributed | TargetState::Failed | TargetState::Applying,
                TargetState::Applying
            ) | (
                TargetState::Applying | TargetState::Implemented,
                TargetState::Implemented
            ) | (
                TargetState::Implemented | TargetState::Reviewing,
                TargetState::Reviewing
            ) | (
                TargetState::Implemented | TargetState::Reviewing | TargetState::Merged,
                TargetState::Merged
            ) |             (
                TargetState::Pending
                    | TargetState::Applying
                    | TargetState::Distributed
                    | TargetState::Implemented
                    | TargetState::Reviewing
                    | TargetState::Failed,
                TargetState::Failed
            )
        );

        if !allowed {
            bail!(
                "invalid state transition for '{}': {} -> {}",
                id,
                target.state,
                new_state
            );
        }

        target.state = new_state;
        self.updated = now();
        Ok(())
    }

    /// Record the PR URL for a target.
    pub fn set_pr(&mut self, id: &str, pr_url: String) -> Result<()> {
        let target = self
            .targets
            .iter_mut()
            .find(|t| t.id == id)
            .with_context(|| format!("target '{id}' not in status"))?;
        target.pr = Some(pr_url);
        Ok(())
    }

}

fn now() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_ordering() {
        assert!(TargetState::Implemented.is_at_least(TargetState::Distributed));
        assert!(!TargetState::Pending.is_at_least(TargetState::Distributed));
        assert!(!TargetState::Failed.is_at_least(TargetState::Distributed));
    }

    #[test]
    fn valid_transitions() {
        let mut status = PipelineStatus {
            change: "test".into(),
            updated: "now".into(),
            targets: vec![TargetStatus {
                id: "a".into(),
                repo: "r".into(),
                pr: None,
                state: TargetState::Pending,
            }],
        };
        assert!(status.transition("a", TargetState::Distributed).is_ok());
        assert!(status.transition("a", TargetState::Applying).is_ok());
        assert!(status.transition("a", TargetState::Implemented).is_ok());
    }

    #[test]
    fn invalid_transition() {
        let mut status = PipelineStatus {
            change: "test".into(),
            updated: "now".into(),
            targets: vec![TargetStatus {
                id: "a".into(),
                repo: "r".into(),
                pr: None,
                state: TargetState::Pending,
            }],
        };
        assert!(status.transition("a", TargetState::Implemented).is_err());
    }
}
