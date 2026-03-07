use std::fmt;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::pipeline::Pipeline;
use crate::registry::Registry;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TargetState {
    Pending,
    Distributed,
    Applying,
    Implemented,
    Reviewing,
    Merged,
    Failed,
}

impl TargetState {
    /// Ordinal for "at least this far" comparisons in the happy path.
    fn ordinal(self) -> u8 {
        match self {
            Self::Pending => 0,
            Self::Distributed => 1,
            Self::Applying => 2,
            Self::Implemented => 3,
            Self::Reviewing => 4,
            Self::Merged => 5,
            Self::Failed => 0,
        }
    }

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetStatus {
    pub id: String,
    pub repo: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pr: Option<String>,
    pub state: TargetState,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PipelineStatus {
    pub change: String,
    pub updated: String,
    pub targets: Vec<TargetStatus>,
}

impl PipelineStatus {
    pub fn load(path: &Path) -> Result<Self> {
        let content =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        toml::from_str(&content).with_context(|| format!("parsing {}", path.display()))
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

    pub fn save(&self, path: &Path) -> Result<()> {
        let content = toml::to_string_pretty(self).context("serializing status")?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, content).with_context(|| format!("writing {}", path.display()))
    }

    pub fn get(&self, id: &str) -> Option<&TargetStatus> {
        self.targets.iter().find(|t| t.id == id)
    }

    pub fn transition(&mut self, id: &str, new_state: TargetState) -> Result<()> {
        let target = self
            .targets
            .iter_mut()
            .find(|t| t.id == id)
            .with_context(|| format!("target '{id}' not in status"))?;

        let allowed = matches!(
            (target.state, new_state),
            (TargetState::Pending, TargetState::Distributed)
                | (TargetState::Distributed, TargetState::Applying)
                | (TargetState::Applying, TargetState::Implemented)
                | (TargetState::Applying, TargetState::Failed)
                | (TargetState::Implemented, TargetState::Reviewing)
                | (TargetState::Implemented, TargetState::Merged)
                | (TargetState::Distributed, TargetState::Failed)
                | (TargetState::Implemented, TargetState::Failed)
                | (TargetState::Reviewing, TargetState::Failed)
                | (TargetState::Reviewing, TargetState::Merged)
                // Idempotent re-runs
                | (TargetState::Failed, TargetState::Distributed)
                | (TargetState::Failed, TargetState::Applying)
                | (TargetState::Distributed, TargetState::Distributed)
                | (TargetState::Applying, TargetState::Applying)
                | (TargetState::Implemented, TargetState::Implemented)
                | (TargetState::Reviewing, TargetState::Reviewing)
                | (TargetState::Merged, TargetState::Merged)
                | (TargetState::Failed, TargetState::Failed)
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

    pub fn set_pr(&mut self, id: &str, pr_url: String) -> Result<()> {
        let target = self
            .targets
            .iter_mut()
            .find(|t| t.id == id)
            .with_context(|| format!("target '{id}' not in status"))?;
        target.pr = Some(pr_url);
        Ok(())
    }

    pub fn print_summary(&self) {
        println!("change: {}", self.change);
        println!("updated: {}", self.updated);
        println!();
        println!("{:<24} {:<14} PR", "TARGET", "STATE");
        println!("{}", "-".repeat(72));
        for t in &self.targets {
            println!(
                "{:<24} {:<14} {}",
                t.id,
                t.state,
                t.pr.as_deref().unwrap_or("-")
            );
        }
        let done = self
            .targets
            .iter()
            .filter(|t| t.state.is_at_least(TargetState::Implemented))
            .count();
        println!();
        println!(
            "progress: {}/{} targets implemented or later",
            done,
            self.targets.len()
        );
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
